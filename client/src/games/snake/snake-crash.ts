import { type SnakeView, SnakeWallCollisionMode } from "@/proto/minigames/v2/snake_pb";

import type { CellPosition } from "./snake-geometry";

/** A crash the rules worked out (`SnakeView.crashes`), as the board draws it. */
export interface Crash {
  /** In cell units, a cell's centre at `x + 0.5`. */
  readonly at: CellPosition;
  readonly slots: readonly number[];
  readonly headOn: boolean;
  /** Happened in the tick being shown: the crash plays out now. */
  readonly fresh: boolean;
}

/** How far a crash into a wall is drawn inside the field, so the canvas edge does not cut it. */
const WALL_INSET = 0.1;

export function crashesOf(view: SnakeView, previous: SnakeView | null): Crash[] {
  const onWall = view.wallCollisionMode !== SnakeWallCollisionMode.WRAP_AROUND;
  const inside = (half: number, size: number): number => {
    const at = half / 2;
    if (!onWall) return at;
    if (at <= 0) return WALL_INSET;
    if (at >= size) return size - WALL_INSET;
    return at;
  };
  return view.crashes.map((crash) => ({
    at: { x: inside(crash.x2, view.fieldWidth), y: inside(crash.y2, view.fieldHeight) },
    slots: crash.slots,
    headOn: crash.headOn,
    fresh: crash.slots.some((slot) => previous?.snakes[slot]?.alive === true),
  }));
}
