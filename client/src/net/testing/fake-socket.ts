import { fromJsonString, toJsonString } from "@bufbuild/protobuf";

import {
  type ClientMessage,
  ClientMessageSchema,
  type ServerMessage,
  ServerMessageSchema,
} from "@/proto/minigames/v2/protocol_pb";

/** Records sockets the session opens; the test plays the server. */
export class FakeSocket {
  static opened: FakeSocket[] = [];
  static readonly OPEN = 1;
  readyState = 0;
  readonly sent: string[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(readonly url: string) {
    FakeSocket.opened.push(this);
  }

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    if (this.readyState === 3) return;
    this.readyState = 3;
    this.onclose?.();
  }

  accept(): void {
    this.readyState = FakeSocket.OPEN;
    this.onopen?.();
  }

  lastRequestId(): number {
    return this.lastSent().requestId;
  }

  lastSent(): ClientMessage {
    const last = this.sent.at(-1);
    if (!last) throw new Error("nothing sent");
    return fromJsonString(ClientMessageSchema, last);
  }

  push(message: ServerMessage): void {
    this.onmessage?.({ data: toJsonString(ServerMessageSchema, message) });
  }
}
