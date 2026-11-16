import { shallowRef, type ShallowRef } from "vue";

import type { GameView } from "@/proto/minigames/v2/game_pb";
import type { MatchCatchUp, MatchResult, TickFrame } from "@/proto/minigames/v2/match_pb";
import { Mirror } from "@/wasm/core";

import type { GameCommandInit, MatchSource } from "./match-source";
import { PlaybackClock } from "./playback-clock";
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
  /** Ended by the authority without a result (server error); there is no replay. */
  readonly aborted = shallowRef(false);
  readonly tickMs: number;

  private mirror: Mirror;
  private readonly clock: PlaybackClock;
  private resyncing = false;
  private readonly unsubscribe: (() => void)[] = [];

  constructor(readonly source: MatchSource) {
    this.mirror = Mirror.forHeader(source.header);
    // From the game itself: local matches have no server to fill the header's tick_ms.
    this.tickMs = this.mirror.tickMs;
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
      source.on("ended", (result) => {
        this.result.value = result ?? (this.mirror.isFinished ? this.mirror.result() : null);
        this.aborted.value = this.result.value === null;
        this.ended.value = true;
      }),
    );
    this.publish();
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
    this.publish();
  }

  private publish(at = performance.now()): void {
    this.previous.value = this.current.value;
    this.current.value = { view: this.mirror.view(this.source.mySlot), at };
  }
}
