import { SnakeDirection } from "@/proto/minigames/v2/snake_pb";

export interface GridCell {
  readonly x: number;
  readonly y: number;
}

/** A point in cell units that may lie between cells or outside the field while wrapping. */
export interface CellPosition {
  readonly x: number;
  readonly y: number;
}

export interface FieldSize {
  readonly width: number;
  readonly height: number;
}

export interface GridStep {
  readonly dx: number;
  readonly dy: number;
}

const MOVING_DIRECTIONS = [
  SnakeDirection.UP,
  SnakeDirection.DOWN,
  SnakeDirection.LEFT,
  SnakeDirection.RIGHT,
] as const;

export function stepOf(direction: SnakeDirection): GridStep {
  switch (direction) {
    case SnakeDirection.UP:
      return { dx: 0, dy: -1 };
    case SnakeDirection.DOWN:
      return { dx: 0, dy: 1 };
    case SnakeDirection.LEFT:
      return { dx: -1, dy: 0 };
    case SnakeDirection.RIGHT:
      return { dx: 1, dy: 0 };
    default:
      return { dx: 0, dy: 0 };
  }
}

function wrap(value: number, size: number): number {
  return ((value % size) + size) % size;
}

/**
 * Direction of the single step from `from` to `to`, counting a step across the field border as
 * adjacent (wrap-around fields). `null` when the cells are not neighbours.
 */
export function directionBetween(from: GridCell, to: GridCell, field: FieldSize): SnakeDirection | null {
  for (const direction of MOVING_DIRECTIONS) {
    const step = stepOf(direction);
    if (wrap(from.x + step.dx, field.width) === to.x && wrap(from.y + step.dy, field.height) === to.y)
      return direction;
  }
  return null;
}

export function sameCell(first: GridCell, second: GridCell): boolean {
  return first.x === second.x && first.y === second.y;
}

export function shifted(cell: GridCell, direction: SnakeDirection, distance: number): CellPosition {
  const step = stepOf(direction);
  return { x: cell.x + step.dx * distance, y: cell.y + step.dy * distance };
}

/**
 * Every placement of a one-cell sprite at `position` that overlaps the field: the position itself
 * plus its copies on the opposite border when it sticks out, so a head crossing the edge of a
 * wrap-around field shows on both sides instead of streaking across the board.
 */
export function wrappedCopies(position: CellPosition, field: FieldSize): CellPosition[] {
  const xs = [position.x];
  if (position.x < 0) xs.push(position.x + field.width);
  if (position.x + 1 > field.width) xs.push(position.x - field.width);
  const ys = [position.y];
  if (position.y < 0) ys.push(position.y + field.height);
  if (position.y + 1 > field.height) ys.push(position.y - field.height);
  return xs.flatMap((x) => ys.map((y) => ({ x, y })));
}
