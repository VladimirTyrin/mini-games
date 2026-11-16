import { describe, expect, it } from "vitest";

import { Puzzle2048Direction } from "@/proto/minigames/v2/puzzle2048_pb";

import { directionFromKey, directionFromSwipe } from "./move-direction";

describe("directionFromKey", () => {
  it.each([
    ["ArrowUp", Puzzle2048Direction.UP],
    ["ArrowLeft", Puzzle2048Direction.LEFT],
    ["s", Puzzle2048Direction.DOWN],
    ["D", Puzzle2048Direction.RIGHT],
  ])("%s", (key, expected) => {
    expect(directionFromKey(key)).toBe(expected);
  });

  it.each(["h", "Enter", " ", "arrowup"])("ignores %j", (key) => {
    expect(directionFromKey(key)).toBeNull();
  });
});

describe("directionFromSwipe", () => {
  it("maps swipes to moves towards the swiped edge", () => {
    expect(directionFromSwipe("up")).toBe(Puzzle2048Direction.UP);
    expect(directionFromSwipe("right")).toBe(Puzzle2048Direction.RIGHT);
  });
});
