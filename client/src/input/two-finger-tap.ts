/** The longest a two-finger tap may last, from the first finger down to the last one up. */
const MAX_TAP_MS = 400;
/** How far a finger may wander and still tap, in CSS pixels. */
const MAX_TRAVEL_PX = 24;

interface TouchPoint {
  readonly id: number;
  readonly x: number;
  readonly y: number;
}

/**
 * Tells a tap with two fingers (both down, both up, quickly, hardly moving) from everything
 * else a hand does on a board: one finger (a swipe, a tap), three, a pinch or a drag.
 */
export class TwoFingerTap {
  private startedAt = 0;
  private readonly starts = new Map<number, TouchPoint>();
  private fingers = 0;
  private spoiled = false;

  /** Fingers down: `touches` is every finger on the screen now. */
  down(touches: readonly TouchPoint[], at: number): void {
    if (this.starts.size === 0) {
      this.startedAt = at;
      this.spoiled = false;
      this.fingers = 0;
    }
    for (const touch of touches) if (!this.starts.has(touch.id)) this.starts.set(touch.id, touch);
    this.fingers = Math.max(this.fingers, touches.length);
    if (this.fingers > 2) this.spoiled = true;
  }

  moved(touches: readonly TouchPoint[]): void {
    for (const touch of touches) {
      const start = this.starts.get(touch.id);
      if (start && Math.hypot(touch.x - start.x, touch.y - start.y) > MAX_TRAVEL_PX) this.spoiled = true;
    }
  }

  /** Fingers up: `remaining` is every finger still down. `true` once a two-finger tap ended. */
  up(remaining: readonly TouchPoint[], at: number): boolean {
    if (remaining.length > 0) return false;
    const tapped = !this.spoiled && this.fingers === 2 && at - this.startedAt <= MAX_TAP_MS;
    this.starts.clear();
    return tapped;
  }
}
