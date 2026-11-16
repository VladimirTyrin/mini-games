import { clamp01, easeOutSine } from "@/render/easing";

/** Shortest and longest tween, in steps: tiny corrections still read as motion, far ones don't crawl. */
const MIN_STEPS = 0.5;
const MAX_STEPS = 2;

/**
 * Horizontal position of a worker drawn independently of the tick timeline: the own worker starts
 * walking when the player presses (prediction) and later corrections continue from wherever it
 * is drawn, so neither the prediction nor its confirmation ever snaps.
 */
export class WalkTween {
  private from = 0;
  private to = 0;
  private startAt = 0;
  private durationMs = 0;
  private placed = false;

  constructor(private readonly stepMs: number) {}

  /** Where the tween ends; `null` until placed. */
  get target(): number | null {
    return this.placed ? this.to : null;
  }

  place(x: number): void {
    this.from = x;
    this.to = x;
    this.durationMs = 0;
    this.placed = true;
  }

  reset(): void {
    this.placed = false;
  }

  /** Walks from wherever it is drawn at `startAt` to `x`. */
  walkTo(x: number, startAt: number): void {
    if (!this.placed) {
      this.place(x);
      return;
    }
    const from = this.valueAt(startAt);
    this.from = from;
    this.to = x;
    this.startAt = startAt;
    const steps = Math.min(MAX_STEPS, Math.max(MIN_STEPS, Math.abs(x - from)));
    this.durationMs = this.stepMs * steps;
  }

  progressAt(now: number): number {
    return this.durationMs <= 0 ? 1 : clamp01((now - this.startAt) / this.durationMs);
  }

  valueAt(now: number): number {
    return this.from + (this.to - this.from) * easeOutSine(this.progressAt(now));
  }

  isMovingAt(now: number): boolean {
    return this.from !== this.to && this.progressAt(now) < 1;
  }
}
