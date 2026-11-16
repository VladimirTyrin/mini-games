import type { SwipeDirection } from "@/input/use-swipe";
import { Puzzle2048Direction } from "@/proto/minigames/v2/puzzle2048_pb";

const DIRECTION_BY_KEY: Readonly<Record<string, Puzzle2048Direction>> = {
  ArrowUp: Puzzle2048Direction.UP,
  ArrowDown: Puzzle2048Direction.DOWN,
  ArrowLeft: Puzzle2048Direction.LEFT,
  ArrowRight: Puzzle2048Direction.RIGHT,
  w: Puzzle2048Direction.UP,
  s: Puzzle2048Direction.DOWN,
  a: Puzzle2048Direction.LEFT,
  d: Puzzle2048Direction.RIGHT,
};

const DIRECTION_BY_SWIPE: Readonly<Record<SwipeDirection, Puzzle2048Direction>> = {
  up: Puzzle2048Direction.UP,
  down: Puzzle2048Direction.DOWN,
  left: Puzzle2048Direction.LEFT,
  right: Puzzle2048Direction.RIGHT,
};

/** Arrow keys and WASD (either case, so Caps Lock does not break it); `null` for other keys. */
export function directionFromKey(key: string): Puzzle2048Direction | null {
  return DIRECTION_BY_KEY[key] ?? DIRECTION_BY_KEY[key.toLowerCase()] ?? null;
}

export function directionFromSwipe(swipe: SwipeDirection): Puzzle2048Direction {
  return DIRECTION_BY_SWIPE[swipe];
}
