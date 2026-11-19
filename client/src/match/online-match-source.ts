import { create } from "@bufbuild/protobuf";

import { ErrorCode } from "@/proto/minigames/v2/protocol_pb";

import { GameCommandSchema, GameSecretCommandSchema } from "@/proto/minigames/v2/game_pb";
import type { MatchHeader } from "@/proto/minigames/v2/match_pb";
import { Emitter } from "@/net/emitter";
import { RequestError } from "@/net/request-error";
import type { Session } from "@/net/session";

import { CommandRejectedError } from "./command-rejected-error";
import type { GameCommandInit, GameSecretCommandInit, MatchSource, MatchSourceEvents } from "./match-source";

/** The server allows one catch-up per second per session. */
const CATCH_UP_RETRY_MS = 1_200;

/** Server-authoritative match over the signed-in session. */
export class OnlineMatchSource implements MatchSource {
  readonly kind = "online";
  private readonly events = new Emitter<MatchSourceEvents>();
  private readonly unsubscribe: (() => void)[] = [];
  private isLeft = false;
  /** The server said the match is over: a later "match not found" adds nothing. */
  private hasEnded = false;

  constructor(
    private session: Session,
    readonly header: MatchHeader,
    readonly mySlot: number | null,
  ) {}

  private get matchId(): string {
    return this.header.matchId;
  }

  on<K extends keyof MatchSourceEvents>(
    event: K,
    handler: (payload: MatchSourceEvents[K]) => void,
  ): () => void {
    return this.events.on(event, handler);
  }

  start(): void {
    const session = this.session.events;
    this.unsubscribe.push(
      // Frames carry no match id: once this match is over, they are the next match's.
      session.on("frame", (frame) => {
        if (!this.hasEnded) this.events.emit("frame", frame);
      }),
      session.on("catchUp", (catchUp) => {
        if (catchUp.header?.matchId === this.matchId) this.events.emit("catchUp", catchUp);
      }),
      session.on("paused", (paused) => {
        if (paused.matchId !== this.matchId) return;
        // Servers before the full list only sent the seat that changed.
        const pauses = paused.pauses.length > 0 ? paused.pauses : paused.pause ? [paused.pause] : [];
        this.events.emit("paused", pauses);
      }),
      session.on("resumed", (resumed) => {
        if (resumed.matchId === this.matchId)
          this.events.emit("resumed", { countdownMs: resumed.countdownMs });
      }),
      session.on("seatSecret", (seatSecret) => {
        if (seatSecret.matchId !== this.matchId || !seatSecret.secret) return;
        // A slot says it is some seat's secret for spectators, not this seat's own.
        if (seatSecret.slot !== undefined)
          this.events.emit("spectatorSecret", { slot: seatSecret.slot, secret: seatSecret.secret });
        else this.events.emit("seatSecret", seatSecret.secret);
      }),
      session.on("ended", (ended) => {
        if (ended.matchId !== this.matchId) return;
        this.hasEnded = true;
        this.events.emit("ended", ended.result ?? null);
      }),
      // After a reconnect the server resumes the seat; frames sent meanwhile are lost. The server
      // pushes a catch-up as it does, so this one is mostly a second copy, but its answer is also
      // how a match the server no longer has (MATCH_NOT_FOUND) ends here.
      session.on("signedIn", ({ resumed }) => {
        if (resumed) this.requestCatchUp();
      }),
    );
  }

  /** The account signed in again over a new session ("Continue here"): follow it. */
  rebind(session: Session): void {
    if (this.isLeft || session === this.session) return;
    for (const unsubscribe of this.unsubscribe) unsubscribe();
    this.unsubscribe.length = 0;
    this.session = session;
    this.start();
  }

  async submit(command: GameCommandInit): Promise<void> {
    try {
      await this.session.request({
        case: "submitCommand",
        value: { matchId: this.matchId, command: create(GameCommandSchema, command) },
      });
    } catch (error) {
      if (error instanceof RequestError && error.code === ErrorCode.COMMAND_REJECTED) {
        throw new CommandRejectedError(error.refusal, error.message, { cause: error });
      }
      throw error;
    }
  }

  async submitSecret(command: GameSecretCommandInit): Promise<void> {
    try {
      await this.session.request({
        case: "submitSecretCommand",
        value: { matchId: this.matchId, command: create(GameSecretCommandSchema, command) },
      });
    } catch (error) {
      if (error instanceof RequestError && error.code === ErrorCode.COMMAND_REJECTED) {
        throw new CommandRejectedError(error.refusal, error.message, { cause: error });
      }
      throw error;
    }
  }

  requestCatchUp(): void {
    this.session.request({ case: "requestCatchUp", value: { matchId: this.matchId } }).catch((error) => {
      // The controller ignores frames until the catch-up arrives, so a refused request must be
      // retried; a lost connection triggers its own catch-up on resume.
      if (error instanceof RequestError && error.code === ErrorCode.RATE_LIMITED && !this.isLeft) {
        setTimeout(() => this.requestCatchUp(), CATCH_UP_RETRY_MS);
      }
      // The match is over on the server (it ended or was stopped while this device was away):
      // end it here too rather than wait for frames that never come.
      if (
        error instanceof RequestError &&
        error.code === ErrorCode.MATCH_NOT_FOUND &&
        !this.isLeft &&
        !this.hasEnded
      ) {
        this.hasEnded = true;
        this.events.emit("ended", null);
      }
    });
  }

  async voteDrop(slot: number): Promise<void> {
    await this.session.request({ case: "voteDropPlayer", value: { matchId: this.matchId, slot } });
  }

  leave(): void {
    this.isLeft = true;
    for (const unsubscribe of this.unsubscribe) unsubscribe();
    this.unsubscribe.length = 0;
  }

  async replayFile(): Promise<Uint8Array | null> {
    const reply = await this.session.request({ case: "requestReplay", value: { matchId: this.matchId } });
    return reply.case === "replayData" ? reply.value.replayFile : null;
  }
}
