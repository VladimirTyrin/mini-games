import { create } from "@bufbuild/protobuf";
import { describe, expect, it } from "vitest";

import { SnakeViewSchema, SnakeWallCollisionMode } from "@/proto/minigames/v2/snake_pb";

import { crashesOf } from "./snake-crash";

function view(crashes: { x2: number; y2: number; slots: number[]; headOn: boolean }[], alive: boolean) {
  return create(SnakeViewSchema, {
    fieldWidth: 10,
    fieldHeight: 10,
    wallCollisionMode: SnakeWallCollisionMode.DEATH,
    snakes: [
      { slot: 0, alive },
      { slot: 1, alive },
    ],
    crashes,
  });
}

describe("crashesOf", () => {
  it("draws the rules' crash points in cells, fresh in the tick the snakes died", () => {
    const headOn = { x2: 11, y2: 9, slots: [0, 1], headOn: true };

    expect(crashesOf(view([headOn], false), view([], true))).toEqual([
      { at: { x: 5.5, y: 4.5 }, slots: [0, 1], headOn: true, fresh: true },
    ]);
    expect(crashesOf(view([headOn], false), view([headOn], false))[0]?.fresh).toBe(false);
  });

  it("draws a crash into a wall just inside the field", () => {
    const wall = { x2: 20, y2: 9, slots: [0], headOn: false };

    expect(crashesOf(view([wall], false), null)[0]?.at).toEqual({ x: 9.9, y: 4.5 });
  });
});
