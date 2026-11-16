/** Screen direction a worker looks in: +1 right, -1 left. */
export type Facing = 1 | -1;

/** Workers face where they last moved horizontally; the rules have no notion of facing. */
export class FacingTracker {
  private readonly facings: Facing[] = [];

  constructor(private readonly initial: Facing = 1) {}

  facing(slot: number): Facing {
    return this.facings[slot] ?? this.initial;
  }

  /** A horizontal move by `dx` cells; a vertical-only move keeps the facing. */
  moved(slot: number, dx: number): void {
    if (dx !== 0) this.facings[slot] = dx > 0 ? 1 : -1;
  }

  /** Looks the given way without moving (a press towards a wall or a crate it cannot push). */
  turned(slot: number, facing: Facing): void {
    this.facings[slot] = facing;
  }

  reset(): void {
    this.facings.length = 0;
  }
}
