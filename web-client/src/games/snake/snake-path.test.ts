import { describe, expect, it } from "vitest";

import { SnakeDirection } from "@/proto/minigames/v2/snake_pb";

import { snakeBodyPath } from "./snake-path";

const field = { width: 10, height: 10 };
// Head first, moving right along row 2 after coming up from row 3.
const previousBody = [
  { x: 4, y: 2 },
  { x: 3, y: 2 },
  { x: 3, y: 3 },
];
const movedBody = [
  { x: 5, y: 2 },
  { x: 4, y: 2 },
  { x: 3, y: 2 },
];

describe("snakeBodyPath", () => {
  it("runs through the cell centres of a resting snake, head first", () => {
    const path = snakeBodyPath(previousBody, SnakeDirection.RIGHT, null, field, 0.5);
    expect(path?.points).toEqual([
      { x: 4.5, y: 2.5 },
      { x: 3.5, y: 2.5 },
      { x: 3.5, y: 3.5 },
    ]);
    expect(path?.heading).toBeCloseTo(0);
  });

  it("slides the head forward and pulls the tail along the body", () => {
    const path = snakeBodyPath(movedBody, SnakeDirection.RIGHT, previousBody, field, 0.25);
    expect(path?.points).toEqual([
      { x: 4.75, y: 2.5 },
      { x: 4.5, y: 2.5 },
      { x: 3.5, y: 2.5 },
      { x: 3.5, y: 3.25 },
    ]);
  });

  it("keeps the tail in place while the snake grows", () => {
    const grown = [...movedBody, { x: 3, y: 3 }];
    const path = snakeBodyPath(grown, SnakeDirection.RIGHT, previousBody, field, 0.5);
    expect(path?.points.at(-1)).toEqual({ x: 3.5, y: 3.5 });
  });

  it("continues past a wrap-around border instead of jumping across the field", () => {
    const before = [
      { x: 9, y: 5 },
      { x: 8, y: 5 },
    ];
    const after = [
      { x: 0, y: 5 },
      { x: 9, y: 5 },
    ];
    const path = snakeBodyPath(after, SnakeDirection.RIGHT, before, field, 0.5);
    expect(path?.points[0]).toEqual({ x: 10, y: 5.5 });
  });

  it("turns the head during the first half of a step towards the new direction", () => {
    const turned = [
      { x: 4, y: 1 },
      { x: 4, y: 2 },
      { x: 3, y: 2 },
    ];
    const start = snakeBodyPath(turned, SnakeDirection.UP, previousBody, field, 0);
    const middle = snakeBodyPath(turned, SnakeDirection.UP, previousBody, field, 0.25);
    const turnedFully = snakeBodyPath(turned, SnakeDirection.UP, previousBody, field, 0.5);
    expect(start?.heading).toBeCloseTo(0);
    expect(middle?.heading).toBeCloseTo(-Math.PI / 4);
    expect(turnedFully?.heading).toBeCloseTo(-Math.PI / 2);
  });

  it("draws a one-cell snake as a sliding head", () => {
    const path = snakeBodyPath([{ x: 1, y: 1 }], SnakeDirection.DOWN, [{ x: 1, y: 0 }], field, 0.5);
    expect(path?.points).toEqual([{ x: 1.5, y: 1 }]);
  });
});
