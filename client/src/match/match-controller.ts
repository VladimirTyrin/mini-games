import { shallowRef, type ShallowRef } from "vue";

import type { GameView } from "@/proto/minigames/v2/game_pb";
import type { MatchCatchUp, MatchResult, TickFrame } from "@/proto/minigames/v2/match_pb";
import { Mirror } from "@/wasm/core";

import type { GameCommandInit, MatchSource } from "./match-source";
import { PlaybackClock } from "./playback-clock";
import { resultRevealAt } from "./result-reveal";
import { type SeatPause, seatPauses } from "./seat-pause";

export interface ViewSnapshot {
  view: GameView;
  /** `performance.now()` when showing this view starts; renderers interpolate from it. */
  at: number;
}

/**
 * Applies a source's frames to a WASM mirror and exposes reactive views. On desync (rejected
 * frame or hash mismatch) it asks for a catch-up and rebuilds from scratch.
 */
export class MatchController {
  readonly current: ShallowRef<ViewSnapshot | null> = shallowRef(null);
  readonly previous: ShallowRef<ViewSnapshot | null> = shallowRef(null);
  readonly pauses: ShallowRef<SeatPause[]> = shallowRef([]);
  readonly resumeAt: ShallowRef<number | null> = shallowRef(null);
  readonly result: ShallowRef<MatchResult | null> = shallowRef(null);
  readonly ended = shallowRef(false);
  /** `performance.now()` from which the result may cover the board (the ending plays out first). */
  readonly resultRevealAt: ShallowRef<number | null> = shallowRef(null);
  /** Ended by the authority without a result (server error); there is no replay. */
  readonly aborted = shallowRef(false);
  /** The game's own tick; 0 for turn-based games. */
  readonly gameTickMs: number;
  /** Replays may play faster or slower than the game; 1 everywhere else. */
  private playbackRate = 1;

  private mirror: Mirror;
  private clock: PlaybackClock;
  private resyncing = false;
  private readonly unsubscribe: (() => void)[] = [];

  constructor(readonly source: MatchSource) {
    this.mirror = Mirror.forHeader(source.header);
    // From the game itself: local matches have no server to fill the header's tick_ms.
    this.gameTickMs = this.mirror.tickMs;
    this.clock = new PlaybackClock(this.tickMs);
    this.unsubscribe.push(
      source.on("frame", (frame) => this.onFrame(frame)),
      source.on("catchUp", (catchUp) => this.applyCatchUp(catchUp)),
      source.on("paused", (pauses) => {
        this.clock.reset();
        this.pauses.value = seatPauses(pauses, performance.now());
        this.resumeAt.value = null;
      }),
      source.on("resumed", ({ countdownMs }) => {
        this.pauses.value = [];
        this.resumeAt.value = performance.now() + countdownMs;
      }),
      source.on("rate", (rate) => {
        this.playbackRate = rate;
        // Animations run at the pace frames now arrive at; the old schedule no longer applies.
        this.clock = new PlaybackClock(this.tickMs);
      }),
      source.on("ended", (result) => {
        this.result.value = result ?? (this.mirror.isFinished ? this.mirror.result() : null);
        this.aborted.value = this.result.value === null;
        const now = performance.now();
        this.resultRevealAt.value = this.aborted.value
          ? now
          : resultRevealAt(this.current.value?.at ?? null, this.tickMs, now);
        this.ended.value = true;
      }),
    );
    this.publish();
  }

  /** Real time between two ticks on screen (the game's tick at the playback speed). */
  get tickMs(): number {
    return this.gameTickMs / this.playbackRate;
  }

  get mySlot(): number | null {
    return this.source.mySlot;
  }

  submit(command: GameCommandInit): Promise<void> {
    return this.source.submit(command);
  }

  dispose(): void {
    for (const unsubscribe of this.unsubscribe) unsubscribe();
    this.mirror.free();
  }

  private onFrame(frame: TickFrame): void {
    if (this.resyncing) return;
    const expected = BigInt(this.mirror.nextTick);
    if (frame.tick < expected) return;
    try {
      if (frame.tick > expected) throw new Error(`missed frames ${expected}..${frame.tick}`);
      this.mirror.applyFrame(frame);
    } catch (error) {
      console.warn("desync, requesting catch-up", error);
      this.resyncing = true;
      this.source.requestCatchUp();
      return;
    }
    this.publish(this.clock.showAt(Number(frame.tick), performance.now()));
    if (this.mirror.isFinished && !this.ended.value) {
      this.result.value = this.mirror.result();
    }
  }

  /** Replaces everything applied so far with the authority's full history. */
  applyCatchUp(catchUp: MatchCatchUp): void {
    const header = catchUp.header ?? this.source.header;
    this.mirror.free();
    this.mirror = Mirror.forHeader(header);
    try {
      this.mirror.applyCatchUp(catchUp);
    } catch (error) {
      console.error("catch-up does not replay", error);
    }
    this.resyncing = false;
    const now = performance.now();
    const pauses = catchUp.pauses.length > 0 ? catchUp.pauses : catchUp.pause ? [catchUp.pause] : [];
    this.pauses.value = seatPauses(pauses, now);
    this.resumeAt.value = catchUp.resumeCountdownMs > 0 ? now + catchUp.resumeCountdownMs : null;
    this.previous.value = null;
    this.clock.reset();
    // Seeking a replay back from its end: the match is running again on screen.
    if (!this.mirror.isFinished && this.ended.value) {
      this.ended.value = false;
      this.result.value = null;
      this.resultRevealAt.value = null;
    }
    this.publish();
  }

  private publish(at = performance.now()): void {
    this.previous.value = this.current.value;
    this.current.value = { view: this.mirror.view(this.source.mySlot), at };
  }
}
