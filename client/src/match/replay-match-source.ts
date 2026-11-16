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

  constructor(bytes: Uint8Array) {
    this.endTick = Mirror.replayEndTick(bytes);
    this.file = fromBinary(ReplayFileSchema, bytes);
    if (!this.file.header) throw new Error("replay has no header");
    this.header = this.file.header;
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

  setSpeed(percent: number): void {
    this.speedPercent = Math.min(400, Math.max(25, percent));
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

  requestCatchUp(): void {
    this.seek(this.current);
  }

  voteDrop(): Promise<void> {
    return Promise.resolve();
  }

  leave(): void {
    this.pause();
  }

  replayFile(): Promise<Uint8Array | null> {
    return Promise.resolve(null);
  }

  private schedule(): void {
    if (!this.playing) return;
    const baseMs = this.header.tickMs > 0 ? this.header.tickMs : TURN_BASED_STEP_MS;
    this.timer = setTimeout(
      () => {
        this.emitNext();
        this.schedule();
      },
      (baseMs * 100) / this.speedPercent,
    );
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
