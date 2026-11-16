import type { SnakeDirection } from "@/proto/minigames/v2/snake_pb";

import {
  type CellPosition,
  directionBetween,
  type FieldSize,
  type GridCell,
  sameCell,
  stepOf,
} from "./snake-geometry";

export interface SnakeBodyPath {
  /**
   * Centre line through the cells, head first, in cell units. Consecutive points are unwrapped
   * (a step across a wrap-around border continues past the edge), so the line may leave the
   * field; the renderer draws shifted copies for that.
   */
  readonly points: readonly CellPosition[];
  /** Where the head looks, in radians (0 = right, y grows downwards). */
  readonly heading: number;
}

const centre = (cell: GridCell): CellPosition => ({ x: cell.x + 0.5, y: cell.y + 0.5 });

function angleOf(direction: SnakeDirection): number {
  const step = stepOf(direction);
  return Math.atan2(step.dy, step.dx);
}

/** Points of `cells` from the first one on, each a unit step from the previous one. */
function unwrapped(cells: readonly GridCell[], field: FieldSize): CellPosition[] {
  const [first] = cells;
  if (!first) return [];
  const points = [centre(first)];
  for (let index = 1; index < cells.length; index++) {
    const from = cells[index - 1];
    const to = cells[index];
    const previous = points[index - 1];
    if (!from || !to || !previous) break;
    const direction = directionBetween(from, to, field);
    // Not neighbours (should not happen in a valid body): stop instead of drawing a jump.
    if (direction === null) break;
    const step = stepOf(direction);
    points.push({ x: previous.x + step.dx, y: previous.y + step.dy });
  }
  return points;
}

function lerpPoint(from: CellPosition, to: CellPosition, progress: number): CellPosition {
  return { x: from.x + (to.x - from.x) * progress, y: from.y + (to.y - from.y) * progress };
}

/** Turns the head over the first half of the step, along the shorter way round. */
function turningAngle(from: number, to: number, progress: number): number {
  const t = Math.min(1, progress * 2);
  const eased = t * t * (3 - 2 * t);
  const delta = Math.atan2(Math.sin(to - from), Math.cos(to - from));
  return from + delta * eased;
}

function stepTaken(
  previousBody: readonly GridCell[],
  body: readonly GridCell[],
  field: FieldSize,
): SnakeDirection | null {
  const [previousHead] = previousBody;
  const [head, neck] = body;
  if (!previousHead || !head) return null;
  if (neck && !sameCell(neck, previousHead)) return null;
  return directionBetween(previousHead, head, field);
}

/**
 * The snake `progress` (0..1) of the way from `previousBody` to `body`: the head slides into its
 * new cell and the tail slides along the body, so the line moves continuously and a turn grows
 * into a curve. Without a usable previous body (first frame, catch-up, no step) it is at rest.
 */
export function snakeBodyPath(
  body: readonly GridCell[],
  heading: SnakeDirection,
  previousBody: readonly GridCell[] | null,
  field: FieldSize,
  progress: number,
): SnakeBodyPath | null {
  const step = previousBody ? stepTaken(previousBody, body, field) : null;
  if (previousBody && step !== null) return movingPath(previousBody, body, step, field, progress);
  return restingPath(body, heading, field);
}

function restingPath(
  body: readonly GridCell[],
  heading: SnakeDirection,
  field: FieldSize,
): SnakeBodyPath | null {
  const points = unwrapped(body, field);
  const [head, neck] = points;
  if (!head) return null;
  return {
    points,
    heading: neck ? Math.atan2(head.y - neck.y, head.x - neck.x) : angleOf(heading),
  };
}

function movingPath(
  previousBody: readonly GridCell[],
  body: readonly GridCell[],
  step: SnakeDirection,
  field: FieldSize,
  progress: number,
): SnakeBodyPath | null {
  const previous = unwrapped(previousBody, field);
  const [previousHead, previousNeck] = previous;
  if (!previousHead) return null;
  const offset = stepOf(step);
  const head = { x: previousHead.x + offset.dx * progress, y: previousHead.y + offset.dy * progress };
  const grew = body.length > previousBody.length;
  const trailing = [...previous];
  if (!grew) {
    const tail = trailing.pop();
    const beforeTail = trailing.at(-1);
    if (tail && beforeTail) trailing.push(lerpPoint(tail, beforeTail, progress));
  }
  const target = angleOf(step);
  const before = previousNeck
    ? Math.atan2(previousHead.y - previousNeck.y, previousHead.x - previousNeck.x)
    : target;
  return { points: [head, ...trailing], heading: turningAngle(before, target, progress) };
}
