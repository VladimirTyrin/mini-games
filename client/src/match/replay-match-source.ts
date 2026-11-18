import { create, fromBinary } from "@bufbuild/protobuf";

import {
  MatchCatchUpSchema,
  type MatchHeader,
  type TickFrame,
  TickFrameSchema,
} from "@/proto/minigames/v2/match_pb";
import { type ReplayFile, ReplayFileSchema } from "@/proto/minigames/v2/replay_pb";
import { Emitter } from "@/net/emitter";
import { Mirror } from "@/wasm/core";

import type { MatchSource, MatchSourceEvents } from "./match-source";

const TURN_BASED_STEP_MS = 700;

/**
 * Plays a replay file client-side. The file is validated by the core first (format, settings,
 * bounded frame log), so `endTick` is trustworthy. Empty frames are created per tick on the fly,
 * never all at once. Seeking replays from the start through a catch-up.
 */
export class ReplayMatchSource implements MatchSource {
  readonly kind = "replay";
  readonly mySlot = null;
  readonly header: MatchHeader;
  private readonly events = new Emitter<MatchSourceEvents>();
  private readonly file: ReplayFile;
  private readonly endTick: bigint;
  /** Number of ticks emitted so far = the next tick to emit. */
  private position = 0n;
  /** Index into `file.frames` of the next logged frame. */
  private nextLogged = 0;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private playing = false;
  private speedPercent = 100;
  private readonly progressHandlers = new Set<() => void>();
  /** The game's own tick; 0 for turn-based games. */
  readonly gameTickMs: number;

  constructor(private readonly bytes: Uint8Array) {
    this.endTick = Mirror.replayEndTick(bytes);
    this.file = fromBinary(ReplayFileSchema, bytes);
    if (!this.file.header) throw new Error("replay has no header");
    this.header = this.file.header;
    // From the game, not the header: a local match's header has tick_ms 0, which played a
    // real-time replay at the turn-based pace.
    const mirror = Mirror.forHeader(this.header);
    try {
      this.gameTickMs = mirror.tickMs;
    } finally {
      mirror.free();
    }
  }

  /** Real time between two frames at the current speed. */
  get frameMs(): number {
    return ((this.gameTickMs > 0 ? this.gameTickMs : TURN_BASED_STEP_MS) * 100) / this.speedPercent;
  }

  get playbackRate(): number {
    return this.speedPercent / 100;
  }

  get isTurnBased(): boolean {
    return this.gameTickMs === 0;
  }

  get length(): number {
    return Number(this.endTick);
  }

  get current(): number {
    return Number(this.position);
  }

  get isPlaying(): boolean {
    return this.playing;
  }

  get speed(): number {
    return this.speedPercent;
  }

  onProgress(handler: () => void): () => void {
    this.progressHandlers.add(handler);
    return () => this.progressHandlers.delete(handler);
  }

  on<K extends keyof MatchSourceEvents>(
    event: K,
    handler: (payload: MatchSourceEvents[K]) => void,
  ): () => void {
    return this.events.on(event, handler);
  }

  start(): void {
    this.play();
  }

  play(): void {
    if (this.position >= this.endTick) this.seek(0);
    this.playing = true;
    this.schedule();
    this.notify();
  }

  pause(): void {
    this.playing = false;
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    this.notify();
  }

  step(): void {
    this.pause();
    this.emitNext();
  }

  stepBack(): void {
    this.pause();
    this.seek(this.current - 1);
  }

  restart(): void {
    this.seek(0);
    this.play();
  }

  setSpeed(percent: number): void {
    const clamped = Math.min(400, Math.max(25, percent));
    if (clamped === this.speedPercent) return;
    this.speedPercent = clamped;
    this.events.emit("rate", this.playbackRate);
    // The next frame keeps to the new pace instead of the old one's pending timer.
    if (this.playing) {
      if (this.timer) clearTimeout(this.timer);
      this.schedule();
    }
    this.notify();
  }

  /** Jumps to the state after `tickCount` ticks. */
  seek(tickCount: number): void {
    const target = BigInt(Math.min(Math.max(0, Math.floor(tickCount)), this.length));
    const logged = this.file.frames.filter((frame) => frame.tick < target);
    this.position = target;
    this.nextLogged = logged.length;
    this.events.emit(
      "catchUp",
      create(MatchCatchUpSchema, { header: this.header, frames: logged, nextTick: target }),
    );
    this.notify();
  }

  submit(): Promise<void> {
    return Promise.reject(new Error("replays are read-only"));
  }

  submitSecret(): Promise<void> {
    return Promise.reject(new Error("replays are read-only"));
  }

  requestCatchUp(): void {
    this.seek(this.current);
  }

  voteDrop(): Promise<void> {
    return Promise.resolve();
  }

  leave(): void {
    this.pause();
  }

  /** The file itself: a board may read ahead in it (Sea Battle shows the fleets from the start). */
  replayFile(): Promise<Uint8Array | null> {
    return Promise.resolve(this.bytes);
  }

  private schedule(): void {
    if (!this.playing) return;
    this.timer = setTimeout(() => {
      this.emitNext();
      this.schedule();
    }, this.frameMs);
  }

  private frameAt(tick: bigint): TickFrame {
    const logged = this.file.frames[this.nextLogged];
    if (logged && logged.tick === tick) {
      this.nextLogged += 1;
      return logged;
    }
    return create(TickFrameSchema, { tick });
  }

  private emitNext(): void {
    if (this.position >= this.endTick) {
      this.playing = false;
      this.events.emit("ended", this.file.result ?? null);
      this.notify();
      return;
    }
    const frame = this.frameAt(this.position);
    this.position += 1n;
    this.events.emit("frame", frame);
    this.notify();
  }

  private notify(): void {
    for (const handler of this.progressHandlers) handler();
  }
}
