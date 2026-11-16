import { describe, expect, it } from "vitest";

import { SnakeDirection } from "@/proto/minigames/v2/snake_pb";

import { poseSnake } from "./snake-pose";

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

describe("poseSnake", () => {
  it("draws a resting snake cell by cell, tail first", () => {
    const pose = poseSnake(previousBody, SnakeDirection.RIGHT, null, field, 0.5);
    expect(pose?.placements).toEqual([
      { sprite: "tailJoinsUp", at: { x: 3, y: 3 } },
      { sprite: "turnDownRight", at: { x: 3, y: 2 } },
      { sprite: "headRight", at: { x: 4, y: 2 } },
    ]);
  });

  it("slides the head forward and retracts the tail inside its old cell", () => {
    const pose = poseSnake(movedBody, SnakeDirection.RIGHT, previousBody, field, 0.25);
    expect(pose?.placements).toEqual([
      { sprite: "tailJoinsUp", at: { x: 3, y: 2.75 }, clipTo: [{ x: 3, y: 3, width: 1, height: 1 }] },
      { sprite: "turnDownRight", at: { x: 3, y: 2 } },
      { sprite: "straightHorizontal", at: { x: 4, y: 2 } },
      { sprite: "headRight", at: { x: 4.25, y: 2 } },
    ]);
    expect(pose?.head).toEqual({ x: 4.25, y: 2 });
  });

  it("slides the tail over a straight piece so its tip stays visible", () => {
    const straight = [
      { x: 5, y: 2 },
      { x: 4, y: 2 },
      { x: 3, y: 2 },
    ];
    const moved = [{ x: 6, y: 2 }, ...straight.slice(0, 2)];
    const pose = poseSnake(moved, SnakeDirection.RIGHT, straight, field, 0.9);
    const [piece, tail] = pose?.placements ?? [];
    // The piece ahead of the tail is cut back to where the tail ends, minus a small overlap.
    expect(piece?.sprite).toBe("straightHorizontal");
    expect(piece?.clipTo?.[0]?.x).toBeCloseTo(4.86);
    expect(piece?.clipTo?.[0]?.width).toBeCloseTo(0.14);
    expect(tail).toEqual({
      sprite: "tailJoinsRight",
      at: { x: 3.9, y: 2 },
      clipTo: [
        { x: 3, y: 2, width: 1, height: 1 },
        { x: 4, y: 2, width: 1, height: 1 },
      ],
    });
  });

  it("keeps the tail in place while the snake grows", () => {
    const grownBody = [...movedBody, { x: 3, y: 3 }];
    const pose = poseSnake(grownBody, SnakeDirection.RIGHT, previousBody, field, 0.5);
    expect(pose?.placements[0]).toEqual({ sprite: "tailJoinsUp", at: { x: 3, y: 3 } });
  });

  it("moves the head across a wrap-around border instead of across the board", () => {
    const before = [
      { x: 9, y: 5 },
      { x: 8, y: 5 },
    ];
    const after = [
      { x: 0, y: 5 },
      { x: 9, y: 5 },
    ];
    const pose = poseSnake(after, SnakeDirection.RIGHT, before, field, 0.5);
    expect(pose?.head).toEqual({ x: 9.5, y: 5 });
    expect(pose?.placements).toContainEqual(
      expect.objectContaining({ sprite: "straightHorizontal", at: { x: 9, y: 5 } }),
    );
  });

  it("rests when the previous body is not one step behind", () => {
    const pose = poseSnake(movedBody, SnakeDirection.RIGHT, [{ x: 0, y: 0 }], field, 0.5);
    expect(pose?.head).toEqual({ x: 5, y: 2 });
  });

  it("draws a one-cell snake as a sliding head", () => {
    const pose = poseSnake([{ x: 1, y: 1 }], SnakeDirection.DOWN, [{ x: 1, y: 0 }], field, 0.5);
    expect(pose?.placements).toEqual([{ sprite: "headDown", at: { x: 1, y: 0.5 } }]);
  });
});
