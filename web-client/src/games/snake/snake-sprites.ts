import { SnakeDirection } from "@/proto/minigames/v2/snake_pb";

/** Edge length of one sprite in `public/sprites.png`. */
export const SPRITE_PIXELS = 64;

/**
 * Cells of the sprite sheet. Heads are named by where they face; tails and body pieces by the
 * sides of the cell where they join the rest of the snake.
 */
export const SNAKE_SPRITE_CELLS = {
  headUp: { column: 3, row: 0 },
  headRight: { column: 4, row: 0 },
  headLeft: { column: 3, row: 1 },
  headDown: { column: 4, row: 1 },
  tailJoinsUp: { column: 3, row: 2 },
  tailJoinsDown: { column: 4, row: 3 },
  tailJoinsLeft: { column: 3, row: 3 },
  tailJoinsRight: { column: 4, row: 2 },
  straightHorizontal: { column: 1, row: 0 },
  straightVertical: { column: 2, row: 1 },
  turnUpLeft: { column: 2, row: 2 },
  turnUpRight: { column: 0, row: 1 },
  turnDownLeft: { column: 2, row: 0 },
  turnDownRight: { column: 0, row: 0 },
  apple: { column: 0, row: 3 },
} as const;

export type SnakeSpriteName = keyof typeof SNAKE_SPRITE_CELLS;

export function headSprite(facing: SnakeDirection): SnakeSpriteName {
  switch (facing) {
    case SnakeDirection.DOWN:
      return "headDown";
    case SnakeDirection.LEFT:
      return "headLeft";
    case SnakeDirection.RIGHT:
      return "headRight";
    default:
      return "headUp";
  }
}

export function tailSprite(towardsBody: SnakeDirection): SnakeSpriteName {
  switch (towardsBody) {
    case SnakeDirection.DOWN:
      return "tailJoinsDown";
    case SnakeDirection.LEFT:
      return "tailJoinsLeft";
    case SnakeDirection.RIGHT:
      return "tailJoinsRight";
    default:
      return "tailJoinsUp";
  }
}

const isVertical = (direction: SnakeDirection) =>
  direction === SnakeDirection.UP || direction === SnakeDirection.DOWN;

/** The piece joining the cell's neighbours on the `first` and `second` sides. */
export function bodySprite(first: SnakeDirection, second: SnakeDirection): SnakeSpriteName {
  if (isVertical(first) === isVertical(second))
    return isVertical(first) ? "straightVertical" : "straightHorizontal";
  const [vertical, horizontal] = isVertical(first) ? [first, second] : [second, first];
  if (vertical === SnakeDirection.UP)
    return horizontal === SnakeDirection.LEFT ? "turnUpLeft" : "turnUpRight";
  return horizontal === SnakeDirection.LEFT ? "turnDownLeft" : "turnDownRight";
}
