import type { ReplayFile } from "@/proto/minigames/v2/replay_pb";
import type { MatchHeader, TickFrame } from "@/proto/minigames/v2/match_pb";
import { Emitter } from "@/net/emitter";
import { expandCompactLog } from "@/wasm/core";

import type { MatchSource, MatchSourceEvents } from "./match-source";

const TURN_BASED_STEP_MS = 700;

/**
 * Plays a replay file client-side. Seeking rebuilds from the start through a catch-up, which the
 * controller already supports; the core replays thousands of frames per millisecond.
 */
export class ReplayMatchSource implements MatchSource {
  readonly kind = "replay";
  readonly mySlot = null;
  readonly header: MatchHeader;
  private readonly events = new Emitter<MatchSourceEvents>();
  private readonly frames: TickFrame[];
  private position = 0;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private playing = false;
  private speedPercent = 100;
  private readonly progressHandlers = new Set<() => void>();

  constructor(private readonly file: ReplayFile) {
    if (!file.header) throw new Error("replay has no header");
    this.header = file.header;
    this.frames = [...expandCompactLog(file.frames, 0n, file.endTick)];
  }

  get length(): number {
    return this.frames.length;
  }

  get current(): number {
    return this.position;
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
    if (this.position >= this.frames.length) this.seek(0);
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

  /** Jumps to `frameCount` applied frames. */
  seek(frameCount: number): void {
    this.position = Math.min(Math.max(0, frameCount), this.frames.length);
    const end = this.frames[this.position - 1]?.tick ?? -1n;
    this.events.emit("catchUp", {
      $typeName: "minigames.v2.MatchCatchUp",
      header: this.header,
      frames: this.frames.slice(0, this.position),
      nextTick: end + 1n,
      pause: undefined,
    });
    this.notify();
  }

  submit(): Promise<void> {
    return Promise.reject(new Error("replays are read-only"));
  }

  requestCatchUp(): void {
    this.seek(this.position);
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

  private emitNext(): void {
    const frame = this.frames[this.position];
    if (!frame) {
      this.playing = false;
      this.events.emit("ended", this.file.result ?? null);
      this.notify();
      return;
    }
    this.position += 1;
    this.events.emit("frame", frame);
    this.notify();
  }

  private notify(): void {
    for (const handler of this.progressHandlers) handler();
  }
}
