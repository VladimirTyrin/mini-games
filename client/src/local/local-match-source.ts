import { create, fromBinary, toBinary } from "@bufbuild/protobuf";

import {
  GameCommandSchema,
  GameSeatSecretSchema,
  GameSecretCommandSchema,
} from "@/proto/minigames/v2/game_pb";
import {
  MatchCatchUpSchema,
  type MatchHeader,
  MatchHeaderSchema,
  TickFrameSchema,
} from "@/proto/minigames/v2/match_pb";
import { ReplayFileSchema } from "@/proto/minigames/v2/replay_pb";
import { CommandRejectedError } from "@/match/command-rejected-error";
import type {
  GameCommandInit,
  GameSecretCommandInit,
  MatchSource,
  MatchSourceEvents,
} from "@/match/match-source";
import { Emitter } from "@/net/emitter";

import type { FromWorker, ToWorker } from "./worker-protocol";

interface Pending {
  resolve: (value: Uint8Array | null) => void;
  reject: (error: Error) => void;
}

/** Match against bots (or alone) with the authority in a Web Worker: no server, no network. */
/** `hidden`: the tab or app is in the background; `player`: the player paused. */
export type PauseReason = "hidden" | "player";

export class LocalMatchSource implements MatchSource {
  readonly kind = "local";
  private readonly events = new Emitter<MatchSourceEvents>();
  private readonly worker: Worker;
  private nextRequestId = 1;
  private readonly pending = new Map<number, Pending>();
  private started = false;
  private resumeTimer: ReturnType<typeof setTimeout> | null = null;
  /** Why the game is held; it runs again only once every reason is gone. */
  private readonly pauseReasons = new Set<PauseReason>();

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
    this.started = true;
    const seed = crypto.getRandomValues(new Uint32Array(2));
    this.send({
      type: "start",
      header: toBinary(MatchHeaderSchema, this.header),
      seedHigh: seed[0] ?? 0,
      seedLow: seed[1] ?? 0,
    });
  }

  submit(command: GameCommandInit): Promise<void> {
    if (!this.started) return Promise.reject(new CommandRejectedError("the match has not started yet"));
    const requestId = this.nextRequestId++;
    const bytes = toBinary(GameCommandSchema, create(GameCommandSchema, command));
    return new Promise<void>((resolve, reject) => {
      this.pending.set(requestId, { resolve: () => resolve(), reject });
      this.send({ type: "submit", requestId, slot: this.mySlot, command: bytes });
    });
  }

  submitSecret(command: GameSecretCommandInit): Promise<void> {
    if (!this.started) return Promise.reject(new CommandRejectedError("the match has not started yet"));
    const requestId = this.nextRequestId++;
    const bytes = toBinary(GameSecretCommandSchema, create(GameSecretCommandSchema, command));
    return new Promise<void>((resolve, reject) => {
      this.pending.set(requestId, { resolve: () => resolve(), reject });
      this.send({ type: "submitSecret", requestId, slot: this.mySlot, command: bytes });
    });
  }

  /**
   * Rebuilds the page's mirror from the worker's full log. Both run the same core, so this only
   * matters if the mirror ever refused a frame; without it the board would freeze silently.
   */
  requestCatchUp(): void {
    this.replayFile()
      .then((bytes) => {
        if (!bytes) return;
        const replay = fromBinary(ReplayFileSchema, bytes);
        this.events.emit(
          "catchUp",
          create(MatchCatchUpSchema, {
            header: replay.header,
            frames: replay.frames,
            nextTick: replay.endTick,
          }),
        );
      })
      .catch((error: unknown) => console.error("local catch-up failed", error));
  }

  voteDrop(): Promise<void> {
    return Promise.resolve();
  }

  /**
   * Holds the game for `reason` (a backgrounded tab or app must not let bots play on; the player
   * may pause too) or lets it go. A real-time game resumes after `countdownMs`, as an online match
   * does, not under your feet.
   */
  setPaused(reason: PauseReason, paused: boolean, countdownMs = 0): void {
    const wasPaused = this.pauseReasons.size > 0;
    if (paused) this.pauseReasons.add(reason);
    else this.pauseReasons.delete(reason);
    const isPaused = this.pauseReasons.size > 0;
    if (isPaused === wasPaused) return;
    if (this.resumeTimer) clearTimeout(this.resumeTimer);
    this.resumeTimer = null;
    if (isPaused) {
      this.send({ type: "pause" });
      this.events.emit("paused", []);
      return;
    }
    if (countdownMs <= 0) {
      this.send({ type: "resume" });
      return;
    }
    this.events.emit("resumed", { countdownMs });
    this.resumeTimer = setTimeout(() => {
      this.resumeTimer = null;
      this.send({ type: "resume" });
    }, countdownMs);
  }

  leave(): void {
    if (this.resumeTimer) clearTimeout(this.resumeTimer);
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
      case "seatSecret":
        this.events.emit("seatSecret", fromBinary(GameSeatSecretSchema, message.secret));
        break;
      case "rejected": {
        const pending = this.pending.get(message.requestId);
        this.pending.delete(message.requestId);
        pending?.reject(new CommandRejectedError(message.message));
        break;
      }
      case "replay":
        this.settle(message.requestId, message.replay);
        break;
      case "ended":
        this.events.emit("ended", null);
        break;
      case "failed":
        // The board would stay empty or frozen: end the match as aborted, which says so.
        console.error("local authority failed:", message.message);
        for (const pending of this.pending.values()) pending.reject(new Error(message.message));
        this.pending.clear();
        this.events.emit("ended", null);
        break;
    }
  }

  private settle(requestId: number, value: Uint8Array | null): void {
    const pending = this.pending.get(requestId);
    this.pending.delete(requestId);
    pending?.resolve(value);
  }
}
