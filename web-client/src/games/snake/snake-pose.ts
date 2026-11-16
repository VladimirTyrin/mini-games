import { SnakeDirection } from "@/proto/minigames/v2/snake_pb";

import {
  type CellPosition,
  directionBetween,
  type FieldSize,
  type GridCell,
  sameCell,
  shifted,
} from "./snake-geometry";
import { bodySprite, headSprite, type SnakeSpriteName, tailSprite } from "./snake-sprites";

/** A rectangle in cell units. */
export interface CellArea {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

export interface SpritePlacement {
  readonly sprite: SnakeSpriteName;
  readonly at: CellPosition;
  /** Draw only inside these areas (a retracting tail must not stick out of the body). */
  readonly clipTo?: readonly CellArea[];
}

const wholeCell = (cell: GridCell): CellArea => ({ x: cell.x, y: cell.y, width: 1, height: 1 });

/** Hides the sprite pixels under the retracting tail; a sliver stays so no seam opens. */
const TAIL_OVERLAP = 0.04;

/** The part of `cell` ahead of a tail that has slid `progress` into it from the tail side. */
function aheadOfTail(cell: GridCell, towardsHead: SnakeDirection, progress: number): CellArea {
  const covered = Math.max(0, progress - TAIL_OVERLAP);
  switch (towardsHead) {
    case SnakeDirection.UP:
      return { x: cell.x, y: cell.y, width: 1, height: 1 - covered };
    case SnakeDirection.DOWN:
      return { x: cell.x, y: cell.y + covered, width: 1, height: 1 - covered };
    case SnakeDirection.LEFT:
      return { x: cell.x, y: cell.y, width: 1 - covered, height: 1 };
    default:
      return { x: cell.x + covered, y: cell.y, width: 1 - covered, height: 1 };
  }
}

export interface SnakePose {
  /** In drawing order: tail first, head last. */
  readonly placements: readonly SpritePlacement[];
  readonly head: CellPosition;
}

/**
 * Sprites of one snake `progress` (0..1) of the way from `previousBody` to `body`. The body cells
 * stay put while the head slides into its new cell and the tail retracts, which keeps every
 * corner intact. Without a usable previous body (first frame, catch-up, no movement) the snake
 * is drawn at rest.
 */
export function poseSnake(
  body: readonly GridCell[],
  heading: SnakeDirection,
  previousBody: readonly GridCell[] | null,
  field: FieldSize,
  progress: number,
): SnakePose | null {
  const step = previousBody ? stepTaken(previousBody, body, field) : null;
  if (previousBody && step !== null) return movingPose(previousBody, body, step, field, progress);
  return restingPose(body, heading, field);
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

function link(from: GridCell, to: GridCell, field: FieldSize): SnakeDirection {
  return directionBetween(from, to, field) ?? SnakeDirection.UNSPECIFIED;
}

function restingPose(body: readonly GridCell[], heading: SnakeDirection, field: FieldSize): SnakePose | null {
  const [head, neck] = body;
  if (!head) return null;
  const placements: SpritePlacement[] = [];
  const tailIndex = body.length - 1;
  for (let index = tailIndex; index >= 1; index--) {
    const cell = body[index];
    const towardsHead = body[index - 1];
    if (!cell || !towardsHead) continue;
    const towardsTail = body[index + 1];
    placements.push({
      sprite: towardsTail
        ? bodySprite(link(cell, towardsHead, field), link(cell, towardsTail, field))
        : tailSprite(link(cell, towardsHead, field)),
      at: cell,
    });
  }
  const facing = neck ? (directionBetween(neck, head, field) ?? heading) : heading;
  placements.push({ sprite: headSprite(facing), at: head });
  return { placements, head };
}

function movingPose(
  previousBody: readonly GridCell[],
  body: readonly GridCell[],
  step: SnakeDirection,
  field: FieldSize,
  progress: number,
): SnakePose | null {
  const [head] = body;
  const [previousHead] = previousBody;
  if (!head || !previousHead) return null;
  const grew = body.length > previousBody.length;
  const placements: SpritePlacement[] = [];
  const tailIndex = previousBody.length - 1;
  let retractingTail: SpritePlacement | null = null;
  for (let index = tailIndex; index >= 0; index--) {
    const cell = previousBody[index];
    const towardsHead = index === 0 ? head : previousBody[index - 1];
    if (!cell || !towardsHead) continue;
    const joinsHead = link(cell, towardsHead, field);
    const towardsTail = previousBody[index + 1];
    if (towardsTail) {
      const sprite = bodySprite(joinsHead, link(cell, towardsTail, field));
      const isStraight = sprite === "straightHorizontal" || sprite === "straightVertical";
      if (retractingTail && index === tailIndex - 1 && isStraight) {
        // The tail slides into this straight piece with its round tip in view; the piece is cut
        // back so its square end does not show around the tip.
        const tailCell = previousBody[tailIndex] ?? cell;
        placements.push({ sprite, at: cell, clipTo: [aheadOfTail(cell, joinsHead, progress)] });
        placements.push({ ...retractingTail, clipTo: [wholeCell(tailCell), wholeCell(cell)] });
        retractingTail = null;
        continue;
      }
      placements.push({ sprite, at: cell });
      if (retractingTail && index === tailIndex - 1) {
        // Ahead of a turn the tail would stick out of the bend: it retracts under the turn.
        placements.splice(placements.length - 1, 0, retractingTail);
        retractingTail = null;
      }
    } else if (grew) {
      placements.push({ sprite: tailSprite(joinsHead), at: cell });
    } else if (index > 0) {
      retractingTail = {
        sprite: tailSprite(joinsHead),
        at: shifted(cell, joinsHead, progress),
        clipTo: [wholeCell(cell)],
      };
    }
  }
  const headAt = shifted(previousHead, step, progress);
  placements.push({ sprite: headSprite(step), at: headAt });
  return { placements, head: headAt };
}
