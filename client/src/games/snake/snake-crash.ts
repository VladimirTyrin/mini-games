import {
  SnakeDeathReason,
  type SnakeSeatView,
  type SnakeView,
  SnakeWallCollisionMode,
} from "@/proto/minigames/v2/snake_pb";

import { type CellPosition, type FieldSize, stepOf } from "./snake-geometry";

/** Where snakes crashed, to show it: the rules only say that they died and why. */
export interface Crash {
  /** In cell units, a cell's centre at `x + 0.5`. */
  readonly at: CellPosition;
  readonly slots: readonly number[];
  /** Two or more snakes met there: heads into the same cell, or into each other. */
  readonly headOn: boolean;
  /** Happened in the tick being shown: the crash plays out now. */
  readonly fresh: boolean;
}

const CRASHES = new Set([
  SnakeDeathReason.WALL_COLLISION,
  SnakeDeathReason.SELF_COLLISION,
  SnakeDeathReason.SNAKE_COLLISION,
]);

function wrapped(value: number, size: number): number {
  return ((value % size) + size) % size;
}

/**
 * A dead snake keeps the body it had before the fatal move and the direction of that move, so
 * it hit the edge of the cell in front of its head. Two heads aiming at one cell met in that
 * cell, so a contested cell is the point for both; heads swapping cells meet on the edge between
 * them, which both reach from their own side.
 */
function crashPoint(
  seat: SnakeSeatView,
  contestedTargets: ReadonlySet<string>,
  view: SnakeView,
): CellPosition | null {
  const head = seat.body[0];
  if (!head) return null;
  const step = stepOf(seat.direction);
  const wraps = view.wallCollisionMode === SnakeWallCollisionMode.WRAP_AROUND;
  const target = {
    x: wraps ? wrapped(head.x + step.dx, view.fieldWidth) : head.x + step.dx,
    y: wraps ? wrapped(head.y + step.dy, view.fieldHeight) : head.y + step.dy,
  };
  if (contestedTargets.has(`${target.x},${target.y}`)) return { x: target.x + 0.5, y: target.y + 0.5 };
  // A wall's crash stays a little inside the field, so the canvas edge does not cut it.
  const reach = seat.deathReason === SnakeDeathReason.WALL_COLLISION ? 0.4 : 0.5;
  const edge = { x: head.x + 0.5 + step.dx * reach, y: head.y + 0.5 + step.dy * reach };
  // Across a wrap-around border both sides name the same edge.
  return wraps ? { x: wrapped(edge.x, view.fieldWidth), y: wrapped(edge.y, view.fieldHeight) } : edge;
}

export function crashesOf(view: SnakeView, previous: SnakeView | null): Crash[] {
  const crashed = view.snakes.filter((seat) => !seat.alive && CRASHES.has(seat.deathReason));
  const field: FieldSize = { width: view.fieldWidth, height: view.fieldHeight };
  const targetCount = new Map<string, number>();
  for (const seat of crashed) {
    const head = seat.body[0];
    if (!head || seat.deathReason !== SnakeDeathReason.SNAKE_COLLISION) continue;
    const step = stepOf(seat.direction);
    const key = `${wrapped(head.x + step.dx, field.width)},${wrapped(head.y + step.dy, field.height)}`;
    targetCount.set(key, (targetCount.get(key) ?? 0) + 1);
  }
  const contested = new Set([...targetCount].filter(([, count]) => count > 1).map(([key]) => key));
  const byPoint = new Map<string, { at: CellPosition; slots: number[]; fresh: boolean }>();
  for (const seat of crashed) {
    const at = crashPoint(seat, contested, view);
    if (!at) continue;
    const key = `${at.x},${at.y}`;
    const fresh = previous?.snakes[seat.slot]?.alive === true;
    const crash = byPoint.get(key) ?? { at, slots: [], fresh: false };
    crash.slots.push(seat.slot);
    crash.fresh ||= fresh;
    byPoint.set(key, crash);
  }
  return [...byPoint.values()].map((crash) => ({ ...crash, headOn: crash.slots.length > 1 }));
}
