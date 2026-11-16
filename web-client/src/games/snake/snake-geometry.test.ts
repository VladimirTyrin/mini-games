import { describe, expect, it } from "vitest";

import { SnakeDirection } from "@/proto/minigames/v2/snake_pb";

import { directionBetween, wrappedCopies } from "./snake-geometry";

const field = { width: 10, height: 8 };

describe("directionBetween", () => {
  it("finds the step between neighbouring cells", () => {
    expect(directionBetween({ x: 3, y: 3 }, { x: 4, y: 3 }, field)).toBe(SnakeDirection.RIGHT);
    expect(directionBetween({ x: 3, y: 3 }, { x: 3, y: 2 }, field)).toBe(SnakeDirection.UP);
  });

  it("treats opposite borders as neighbours", () => {
    expect(directionBetween({ x: 9, y: 0 }, { x: 0, y: 0 }, field)).toBe(SnakeDirection.RIGHT);
    expect(directionBetween({ x: 0, y: 0 }, { x: 9, y: 0 }, field)).toBe(SnakeDirection.LEFT);
    expect(directionBetween({ x: 2, y: 0 }, { x: 2, y: 7 }, field)).toBe(SnakeDirection.UP);
  });

  it("rejects cells that are not neighbours", () => {
    expect(directionBetween({ x: 3, y: 3 }, { x: 5, y: 3 }, field)).toBeNull();
    expect(directionBetween({ x: 3, y: 3 }, { x: 3, y: 3 }, field)).toBeNull();
  });
});

describe("wrappedCopies", () => {
  it("keeps a position inside the field as is", () => {
    expect(wrappedCopies({ x: 2.5, y: 3 }, field)).toEqual([{ x: 2.5, y: 3 }]);
  });

  it("repeats a sprite sticking out of a border on the opposite side", () => {
    expect(wrappedCopies({ x: 9.25, y: 3 }, field)).toEqual([
      { x: 9.25, y: 3 },
      { x: -0.75, y: 3 },
    ]);
    expect(wrappedCopies({ x: 4, y: -0.5 }, field)).toEqual([
      { x: 4, y: -0.5 },
      { x: 4, y: 7.5 },
    ]);
  });
});
