export type SwipeDirection = "up" | "down" | "left" | "right";

/**
 * When a swipe counts. `on-release`: one gesture, one swipe, decided when the finger lifts (2048:
 * a move must not fire twice). `while-dragging`: a swipe fires the moment the finger has gone far
 * enough, and the finger may keep going and swipe again without lifting (Snake: steering must not
 * wait for the finger to come up, and circling food is one drawn stroke).
 */
export type SwipeMode = "on-release" | "while-dragging";

export type SwipeOutcome =
  { kind: "swipe"; direction: SwipeDirection } | { kind: "tap" } | { kind: "nothing" };

interface Point {
  readonly x: number;
  readonly y: number;
}

/** How far a finger must travel to swipe, in CSS pixels; less is a tap. */
const MIN_DISTANCE_PX = 24;

function directionBetween(from: Point, to: Point): SwipeDirection | null {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  if (Math.max(Math.abs(dx), Math.abs(dy)) < MIN_DISTANCE_PX) return null;
  if (Math.abs(dx) > Math.abs(dy)) return dx > 0 ? "right" : "left";
  return dy > 0 ? "down" : "up";
}

/** One finger's gesture on a board: where it went, and whether that was a swipe or a tap. */
export class SwipeGesture {
  /** Where the current swipe is measured from: the touch-down point, or the last swipe's end. */
  private origin: Point | null = null;
  private lastSwipe: SwipeDirection | null = null;

  constructor(private readonly mode: SwipeMode) {}

  down(point: Point): void {
    this.origin = point;
    this.lastSwipe = null;
  }

  /** The swipe this move completes, if any (only while dragging). */
  moved(point: Point): SwipeDirection | null {
    if (this.mode !== "while-dragging" || !this.origin) return null;
    const direction = directionBetween(this.origin, point);
    if (!direction) return null;
    this.origin = point;
    // Dragging on along a line keeps swiping the same way: that is one swipe, not many.
    if (direction === this.lastSwipe) return null;
    this.lastSwipe = direction;
    return direction;
  }

  up(point: Point): SwipeOutcome {
    if (!this.origin) return { kind: "nothing" };
    // While dragging, the lift counts only for the stretch since the last swipe (a browser may
    // skip the move events of a quick flick and report only where the finger lifted).
    const direction =
      this.mode === "while-dragging" ? this.moved(point) : directionBetween(this.origin, point);
    const swipedWhileDragging = this.lastSwipe !== null;
    this.cancel();
    if (direction) return { kind: "swipe", direction };
    return swipedWhileDragging ? { kind: "nothing" } : { kind: "tap" };
  }

  cancel(): void {
    this.origin = null;
    this.lastSwipe = null;
  }
}
