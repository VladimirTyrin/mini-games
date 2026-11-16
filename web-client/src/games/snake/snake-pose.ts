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

export interface SpritePlacement {
  readonly sprite: SnakeSpriteName;
  readonly at: CellPosition;
  /** Draw only inside this cell (the tail slides out of sight under the next body piece). */
  readonly clipTo?: GridCell;
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
  for (let index = tailIndex; index >= 0; index--) {
    const cell = previousBody[index];
    const towardsHead = index === 0 ? head : previousBody[index - 1];
    if (!cell || !towardsHead) continue;
    const joinsHead = link(cell, towardsHead, field);
    const towardsTail = previousBody[index + 1];
    if (towardsTail) {
      placements.push({ sprite: bodySprite(joinsHead, link(cell, towardsTail, field)), at: cell });
    } else if (grew) {
      placements.push({ sprite: tailSprite(joinsHead), at: cell });
    } else if (index > 0) {
      placements.push({
        sprite: tailSprite(joinsHead),
        at: shifted(cell, joinsHead, progress),
        clipTo: cell,
      });
    }
  }
  const headAt = shifted(previousHead, step, progress);
  placements.push({ sprite: headSprite(step), at: headAt });
  return { placements, head: headAt };
}
