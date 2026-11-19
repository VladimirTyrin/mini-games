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
import { CommonRefusal, RefusalSchema } from "@/proto/minigames/v2/refusal_pb";
import { ReplayFileSchema } from "@/proto/minigames/v2/replay_pb";
import { CommandRejectedError } from "@/match/command-rejected-error";
import type {
  GameCommandInit,
  GameSecretCommandInit,
  MatchSource,
  MatchSourceEvents,
} from "@/match/match-source";
import { Emitter } from "@/net/emitter";
import { commonRefusal } from "@/match/common-refusal";
import { refusalText } from "@/util/refusal-text";

import type { FromWorker, ToWorker } from "./worker-protocol";

interface Pending {
  resolve: (value: Uint8Array | null) => void;
  reject: (error: Error) => void;
}

/** Match against bots (or alone) with the authority in a Web Worker: no server, no network. */
/**
 * `hidden`: the tab or app is in the background; `player`: the player paused; `help`: the player
 * is reading how to play.
 */
export type PauseReason = "hidden" | "player" | "help";

export class LocalMatchSource implements MatchSource {
  readonly kind = "local";
  private readonly events = new Emitter<MatchSourceEvents>();
  private readonly worker: Worker;
  private nextRequestId = 1;
  private readonly pending = new Map<number, Pending>();
  private started = false;
  /** Why the game is held; it runs again only once every reason is gone. */
  private readonly pauseReasons = new Set<PauseReason>();

  constructor(
    readonly header: MatchHeader,
    /** `null`: the person here watches a table of bots. */
    readonly mySlot: number | null,
  ) {
    this.worker = new Worker(new URL("./local-authority.worker.ts", import.meta.url), { type: "module" });
    this.worker.onmessage = (event: MessageEvent<FromWorker>) => this.onMessage(event.data);
    // The worker script itself failed (did not load, threw outside a message): nothing will
    // answer from it any more.
    this.worker.onerror = (event: ErrorEvent) => {
      event.preventDefault();
      this.onMessage({ type: "failed", message: event.message || "the local game stopped" });
    };
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
      watchedSeats: this.mySlot === null ? this.header.seats.length : 0,
    });
  }

  submit(command: GameCommandInit): Promise<void> {
    if (!this.started)
      return Promise.reject(
        new CommandRejectedError(commonRefusal(CommonRefusal.MALFORMED), "the match has not started yet"),
      );
    if (this.isHeld)
      return Promise.reject(
        new CommandRejectedError(commonRefusal(CommonRefusal.PAUSED), "the game is paused"),
      );
    const slot = this.mySlot;
    if (slot === null)
      return Promise.reject(
        new CommandRejectedError(commonRefusal(CommonRefusal.NOT_PLAYING), "you are watching"),
      );
    const requestId = this.nextRequestId++;
    const bytes = toBinary(GameCommandSchema, create(GameCommandSchema, command));
    return new Promise<void>((resolve, reject) => {
      this.pending.set(requestId, { resolve: () => resolve(), reject });
      this.send({ type: "submit", requestId, slot, command: bytes });
    });
  }

  submitSecret(command: GameSecretCommandInit): Promise<void> {
    if (!this.started)
      return Promise.reject(
        new CommandRejectedError(commonRefusal(CommonRefusal.MALFORMED), "the match has not started yet"),
      );
    if (this.isHeld)
      return Promise.reject(
        new CommandRejectedError(commonRefusal(CommonRefusal.PAUSED), "the game is paused"),
      );
    const slot = this.mySlot;
    if (slot === null)
      return Promise.reject(
        new CommandRejectedError(commonRefusal(CommonRefusal.NOT_PLAYING), "you are watching"),
      );
    const requestId = this.nextRequestId++;
    const bytes = toBinary(GameSecretCommandSchema, create(GameSecretCommandSchema, command));
    return new Promise<void>((resolve, reject) => {
      this.pending.set(requestId, { resolve: () => resolve(), reject });
      this.send({ type: "submitSecret", requestId, slot, command: bytes });
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

  /** Paused for some reason: the game takes no commands. */
  private get isHeld(): boolean {
    return this.pauseReasons.size > 0;
  }

  /**
   * Holds the game for `reason` (a backgrounded tab or app must not let bots play on; the player
   * may pause too) or lets it go once no reason holds it. Alone or with bots there is nobody to
   * warn, so play goes on at once, without an online match's countdown.
   */
  setPaused(reason: PauseReason, paused: boolean): void {
    const wasPaused = this.pauseReasons.size > 0;
    if (paused) this.pauseReasons.add(reason);
    else this.pauseReasons.delete(reason);
    const isPaused = this.pauseReasons.size > 0;
    if (isPaused === wasPaused) return;
    if (isPaused) {
      this.send({ type: "pause" });
      this.events.emit("paused", []);
      return;
    }
    this.send({ type: "resume" });
    this.events.emit("resumed", { countdownMs: 0 });
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
      case "seatSecret":
        this.events.emit("seatSecret", fromBinary(GameSeatSecretSchema, message.secret));
        break;
      case "spectatorSecret":
        this.events.emit("spectatorSecret", {
          slot: message.slot,
          secret: fromBinary(GameSeatSecretSchema, message.secret),
        });
        break;
      case "rejected": {
        const pending = this.pending.get(message.requestId);
        this.pending.delete(message.requestId);
        const refusal = fromBinary(RefusalSchema, message.refusal);
        pending?.reject(new CommandRejectedError(refusal, refusalText(refusal)));
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
