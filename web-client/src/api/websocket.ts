import { toBinary, fromBinary } from "@bufbuild/protobuf";
import {
  type ClientMessage,
  ClientMessageSchema,
  type ServerMessage,
  ServerMessageSchema,
} from "../proto/game_service_pb";

export type ConnectionState =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting";

export type MessageHandler = (message: ServerMessage) => void;
export type ConnectionChangeHandler = (state: ConnectionState) => void;

const MIN_RECONNECT_DELAY_MS = 1000;
const MAX_RECONNECT_DELAY_MS = 30000;

export class WebSocketClient {
  private socket: WebSocket | null = null;
  private url: string = "";
  private connectionState: ConnectionState = "disconnected";
  private reconnectAttempts = 0;
  private reconnectTimeoutId: ReturnType<typeof setTimeout> | null = null;
  private shouldReconnect = false;

  onMessage: MessageHandler | null = null;
  onConnectionChange: ConnectionChangeHandler | null = null;

  get state(): ConnectionState {
    return this.connectionState;
  }

  async connect(url: string): Promise<void> {
    this.url = url;
    this.shouldReconnect = true;
    this.reconnectAttempts = 0;

    return this.createConnection();
  }

  private createConnection(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.setConnectionState(
        this.reconnectAttempts > 0 ? "reconnecting" : "connecting"
      );

      try {
        this.socket = new WebSocket(this.url);
        this.socket.binaryType = "arraybuffer";

        this.socket.onopen = () => {
          this.reconnectAttempts = 0;
          this.setConnectionState("connected");
          resolve();
        };

        this.socket.onclose = () => {
          this.setConnectionState("disconnected");
          this.scheduleReconnect();
        };

        this.socket.onerror = () => {
          if (this.connectionState === "connecting") {
            reject(new Error("WebSocket connection failed"));
          }
        };

        this.socket.onmessage = (event) => {
          this.handleMessage(event);
        };
      } catch (error) {
        this.setConnectionState("disconnected");
        reject(error);
      }
    });
  }

  disconnect(): void {
    this.shouldReconnect = false;
    this.cancelReconnect();

    if (this.socket) {
      this.socket.close();
      this.socket = null;
    }

    this.setConnectionState("disconnected");
  }

  send(message: ClientMessage): void {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      console.warn("WebSocket is not connected. Cannot send message.");
      return;
    }

    const binary = toBinary(ClientMessageSchema, message);
    this.socket.send(binary);
  }

  private handleMessage(event: MessageEvent): void {
    if (!this.onMessage) {
      return;
    }

    try {
      const data = new Uint8Array(event.data as ArrayBuffer);
      const message = fromBinary(ServerMessageSchema, data);
      this.onMessage(message);
    } catch (error) {
      console.error("Failed to deserialize server message:", error);
    }
  }

  private setConnectionState(state: ConnectionState): void {
    if (this.connectionState !== state) {
      this.connectionState = state;
      this.onConnectionChange?.(state);
    }
  }

  private scheduleReconnect(): void {
    if (!this.shouldReconnect) {
      return;
    }

    this.cancelReconnect();

    const delay = Math.min(
      MIN_RECONNECT_DELAY_MS * Math.pow(2, this.reconnectAttempts),
      MAX_RECONNECT_DELAY_MS
    );

    this.reconnectTimeoutId = setTimeout(() => {
      this.reconnectAttempts++;
      this.createConnection().catch(() => {});
    }, delay);
  }

  private cancelReconnect(): void {
    if (this.reconnectTimeoutId !== null) {
      clearTimeout(this.reconnectTimeoutId);
      this.reconnectTimeoutId = null;
    }
  }
}
