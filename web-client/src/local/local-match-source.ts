import { create, fromBinary, toBinary } from "@bufbuild/protobuf";

import { GameCommandSchema } from "@/proto/minigames/v2/game_pb";
import { type MatchHeader, MatchHeaderSchema, TickFrameSchema } from "@/proto/minigames/v2/match_pb";
import type { GameCommandInit, MatchSource, MatchSourceEvents } from "@/match/match-source";
import { Emitter } from "@/net/emitter";

import type { FromWorker, ToWorker } from "./worker-protocol";

interface Pending {
  resolve: (value: Uint8Array | null) => void;
  reject: (error: Error) => void;
}

/** Match against bots (or alone) with the authority in a Web Worker: no server, no network. */
export class LocalMatchSource implements MatchSource {
  readonly kind = "local";
  private readonly events = new Emitter<MatchSourceEvents>();
  private readonly worker: Worker;
  private nextRequestId = 1;
  private readonly pending = new Map<number, Pending>();

  constructor(
    readonly header: MatchHeader,
    readonly mySlot: number,
  ) {
    this.worker = new Worker(new URL("./local-authority.worker.ts", import.meta.url), { type: "module" });
    this.worker.onmessage = (event: MessageEvent<FromWorker>) => this.onMessage(event.data);
  }

  on<K extends keyof MatchSourceEvents>(
    event: K,
    handler: (payload: MatchSourceEvents[K]) => void,
  ): () => void {
    return this.events.on(event, handler);
  }

  start(): void {
    const seed = crypto.getRandomValues(new Uint32Array(2));
    this.send({
      type: "start",
      header: toBinary(MatchHeaderSchema, this.header),
      seedHigh: seed[0] ?? 0,
      seedLow: seed[1] ?? 0,
    });
  }

  submit(command: GameCommandInit): Promise<void> {
    const requestId = this.nextRequestId++;
    const bytes = toBinary(GameCommandSchema, create(GameCommandSchema, command));
    return new Promise<void>((resolve, reject) => {
      this.pending.set(requestId, { resolve: () => resolve(), reject });
      this.send({ type: "submit", requestId, slot: this.mySlot, command: bytes });
    });
  }

  /** The local authority cannot desync from itself; nothing to recover. */
  requestCatchUp(): void {}

  voteDrop(): Promise<void> {
    return Promise.resolve();
  }

  /** Background tabs / backgrounded app: freeze the game instead of letting bots play on. */
  setPaused(paused: boolean): void {
    this.send({ type: paused ? "pause" : "resume" });
  }

  leave(): void {
    this.send({ type: "stop" });
    for (const pending of this.pending.values()) pending.reject(new Error("match closed"));
    this.pending.clear();
  }

  replayFile(): Promise<Uint8Array | null> {
    const requestId = this.nextRequestId++;
    return new Promise((resolve, reject) => {
      this.pending.set(requestId, { resolve, reject });
      this.send({ type: "replay", requestId });
    });
  }

  private send(message: ToWorker): void {
    this.worker.postMessage(message);
  }

  private onMessage(message: FromWorker): void {
    switch (message.type) {
      case "frame":
        this.events.emit("frame", fromBinary(TickFrameSchema, message.frame));
        break;
      case "accepted":
        this.settle(message.requestId, null);
        break;
      case "rejected": {
        const pending = this.pending.get(message.requestId);
        this.pending.delete(message.requestId);
        pending?.reject(new Error(message.message));
        break;
      }
      case "replay":
        this.settle(message.requestId, message.replay);
        break;
      case "ended":
        this.events.emit("ended", null);
        break;
      case "failed":
        console.error("local authority failed:", message.message);
        break;
    }
  }

  private settle(requestId: number, value: Uint8Array | null): void {
    const pending = this.pending.get(requestId);
    this.pending.delete(requestId);
    pending?.resolve(value);
  }
}
