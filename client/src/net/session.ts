import { create, fromJsonString, type MessageInitShape, toJsonString } from "@bufbuild/protobuf";

import type { ServerInfo } from "@/proto/minigames/v2/auth_pb";
import type { Identity } from "@/proto/minigames/v2/identity_pb";
import {
  ClientMessageSchema,
  ErrorCode,
  type ServerMessage,
  ServerMessageSchema,
} from "@/proto/minigames/v2/protocol_pb";

import type { Credentials } from "./credentials";
import { Emitter } from "./emitter";
import { RequestError } from "./request-error";

export const PROTOCOL_VERSION = 2;
const REQUEST_TIMEOUT_MS = 10_000;
/** Often enough for the ping shown in the header to be current; a ping is a few bytes. */
const PING_EVERY_MS = 5_000;
const BACKOFF_MS = [500, 1_000, 2_000, 4_000, 8_000, 15_000];

export type ConnectionState =
  /** Not trying to connect. */
  | "offline"
  | "connecting"
  | "signed-in"
  /** Lost the connection; retrying with backoff. */
  | "reconnecting"
  /** Server speaks another protocol: the app must be updated, retrying is pointless. */
  | "outdated"
  /** Sign-in was refused (bad token, guests disabled). */
  | "rejected"
  /** This session was taken over from another window; reconnecting would take it back. */
  | "replaced";

/** States in which reconnecting cannot help; only a new `connect()` leaves them. */
const FINAL_STATES: ReadonlySet<ConnectionState> = new Set(["outdated", "rejected", "replaced"]);

/** Any client message body; plain objects are fine (protobuf-es init shapes). */
export type ClientPayload = NonNullable<MessageInitShape<typeof ClientMessageSchema>["message"]>;

export type ServerPayload = Exclude<ServerMessage["message"], { case: undefined }>;
type PushEvents = { [P in ServerPayload as P["case"]]: P["value"] };

interface SessionEvents extends PushEvents {
  state: ConnectionState;
  signedIn: {
    identity: Identity;
    resumed: boolean;
    sessionToken: string;
    resumeToken: string;
    serverInfo: ServerInfo | null;
  };
  refused: RequestError;
  /** Round trip of a ping, in milliseconds. */
  latency: number;
}

interface Pending {
  resolve: (payload: ServerPayload) => void;
  reject: (error: RequestError) => void;
  timer: ReturnType<typeof setTimeout>;
}

/**
 * One logical session over a WebSocket: signs in on every (re)connect, correlates requests with
 * replies by request id, pings to detect dead connections, reconnects with backoff + jitter and
 * resumes the server-side session with the resume token.
 */
export class Session {
  readonly events = new Emitter<SessionEvents>();
  private socket: WebSocket | null = null;
  private stateValue: ConnectionState = "offline";
  private nextRequestId = 1;
  private readonly pending = new Map<number, Pending>();
  private resumeToken = "";
  private attempt = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private wanted = false;

  /** `resumeToken` from an earlier session in this tab lets a page reload keep its seat. */
  constructor(
    private readonly url: string,
    private credentials: Credentials,
    resumeToken = "",
  ) {
    this.resumeToken = resumeToken;
  }

  get state(): ConnectionState {
    return this.stateValue;
  }

  connect(): void {
    this.wanted = true;
    if (this.socket || this.stateValue === "outdated") return;
    this.open();
  }

  /** Stops reconnecting and closes; the server keeps the seat for its grace period. */
  close(): void {
    this.wanted = false;
    this.clearTimers();
    this.socket?.close();
    this.socket = null;
    this.failPending(RequestError.disconnected());
    this.setState("offline");
  }

  async signOut(): Promise<void> {
    if (this.stateValue === "signed-in") {
      await this.request({ case: "signOut", value: {} }).catch(() => undefined);
    }
    this.resumeToken = "";
    this.close();
  }

  updateCredentials(credentials: Credentials): void {
    this.credentials = credentials;
  }

  request(message: ClientPayload): Promise<ServerPayload> {
    const socket = this.socket;
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      return Promise.reject(RequestError.disconnected());
    }
    const requestId = this.nextRequestId++;
    const envelope = create(ClientMessageSchema, { requestId, message });
    return new Promise<ServerPayload>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(requestId);
        reject(RequestError.timeout());
      }, REQUEST_TIMEOUT_MS);
      this.pending.set(requestId, { resolve, reject, timer });
      socket.send(toJsonString(ClientMessageSchema, envelope));
    });
  }

  private open(): void {
    this.setState(this.attempt === 0 ? "connecting" : "reconnecting");
    const socket = new WebSocket(this.url);
    this.socket = socket;
    socket.onopen = () => void this.signIn(socket);
    socket.onmessage = (event) => this.onMessage(event.data);
    socket.onclose = () => this.onClose(socket);
    socket.onerror = () => socket.close();
  }

  private async signIn(socket: WebSocket): Promise<void> {
    const credentials = this.credentials;
    const hello = {
      protocolVersion: PROTOCOL_VERSION,
      clientVersion: __APP_VERSION__,
      resumeToken: this.resumeToken,
      credentials:
        credentials.kind === "guest"
          ? { case: "guest" as const, value: { displayName: credentials.displayName } }
          : credentials.kind === "google"
            ? { case: "google" as const, value: { idToken: credentials.idToken } }
            : { case: "session" as const, value: { token: credentials.token } },
    };
    try {
      const reply = await this.request({ case: "hello", value: hello });
      if (reply.case !== "welcome" || socket !== this.socket) return;
      const welcome = reply.value;
      this.resumeToken = welcome.resumeToken;
      this.attempt = 0;
      this.startPing();
      this.setState("signed-in");
      if (welcome.identity) {
        this.events.emit("signedIn", {
          identity: welcome.identity,
          resumed: welcome.resumed,
          sessionToken: welcome.sessionToken,
          resumeToken: welcome.resumeToken,
          serverInfo: welcome.serverInfo ?? null,
        });
      }
    } catch (error) {
      if (!(error instanceof RequestError) || socket !== this.socket) return;
      // No answer is not a refusal: keep the credentials and try again. Nor is a server that
      // cannot check them just now (Google's keys out of reach, a hiccup of its own).
      const transient = error.code === ErrorCode.RATE_LIMITED || error.code === ErrorCode.INTERNAL;
      if (error.failure !== "refused" || transient) {
        this.abandon(socket);
        return;
      }
      if (error.code === ErrorCode.PROTOCOL_UNSUPPORTED) {
        this.wanted = false;
        this.setState("outdated");
      } else if (error.code !== ErrorCode.UNSPECIFIED) {
        this.wanted = false;
        this.setState("rejected");
      }
      this.events.emit("refused", error);
      socket.close();
    }
  }

  private onMessage(data: unknown): void {
    if (typeof data !== "string") return;
    let message: ServerMessage;
    try {
      message = fromJsonString(ServerMessageSchema, data, { ignoreUnknownFields: true });
    } catch {
      return;
    }
    const payload = message.message;
    if (payload.case === undefined) return;
    if (payload.case === "error" && payload.value.code === ErrorCode.SIGNED_IN_ELSEWHERE) {
      this.wanted = false;
      this.setState("replaced");
    }
    const pending = message.requestId === 0 ? undefined : this.pending.get(message.requestId);
    if (pending) {
      this.pending.delete(message.requestId);
      clearTimeout(pending.timer);
      if (payload.case === "error") {
        const { code, message: text, refusal, settingProblem } = payload.value;
        pending.reject(new RequestError(code, text, "refused", refusal, settingProblem));
      } else {
        pending.resolve(payload);
      }
      return;
    }
    this.events.emit(payload.case, payload.value as never);
  }

  private onClose(socket: WebSocket): void {
    if (socket !== this.socket) return;
    this.socket = null;
    this.clearTimers();
    this.failPending(RequestError.disconnected());
    if (!this.wanted) {
      if (!FINAL_STATES.has(this.stateValue)) this.setState("offline");
      return;
    }
    this.setState("reconnecting");
    const base = BACKOFF_MS[Math.min(this.attempt, BACKOFF_MS.length - 1)] ?? 15_000;
    this.attempt += 1;
    // Jitter spreads reconnects after a server restart.
    this.reconnectTimer = setTimeout(() => this.open(), base / 2 + Math.random() * base);
  }

  private startPing(): void {
    const socket = this.socket;
    if (!socket) return;
    const ping = () => {
      const sentAt = performance.now();
      this.request({ case: "ping", value: { clientTimeMs: BigInt(Date.now()) } })
        .then(() => {
          if (socket === this.socket) this.events.emit("latency", Math.round(performance.now() - sentAt));
        })
        .catch((error) => {
          if (error instanceof RequestError && error.failure === "timed-out") this.abandon(socket);
        });
    };
    ping();
    this.pingTimer = setInterval(ping, PING_EVERY_MS);
  }

  /**
   * A socket that stopped answering (network switched, phone asleep) may take minutes to report
   * its close, so it is dropped now and the usual reconnect starts at once.
   */
  private abandon(socket: WebSocket): void {
    if (socket !== this.socket) return;
    socket.onopen = null;
    socket.onmessage = null;
    socket.onclose = null;
    socket.onerror = null;
    socket.close();
    this.onClose(socket);
  }

  private clearTimers(): void {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    if (this.pingTimer) clearInterval(this.pingTimer);
    this.reconnectTimer = null;
    this.pingTimer = null;
  }

  private failPending(error: RequestError): void {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
  }

  private setState(state: ConnectionState): void {
    if (this.stateValue === state) return;
    this.stateValue = state;
    this.events.emit("state", state);
  }
}
