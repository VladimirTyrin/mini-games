import { describe, expect, it } from "vitest";

import { SnakeDirection } from "@/proto/minigames/v2/snake_pb";

import { bodySprite } from "./snake-sprites";

describe("bodySprite", () => {
  it("joins opposite sides with a straight piece", () => {
    expect(bodySprite(SnakeDirection.LEFT, SnakeDirection.RIGHT)).toBe("straightHorizontal");
    expect(bodySprite(SnakeDirection.DOWN, SnakeDirection.UP)).toBe("straightVertical");
  });

  it("joins adjacent sides with a turn regardless of order", () => {
    expect(bodySprite(SnakeDirection.UP, SnakeDirection.LEFT)).toBe("turnUpLeft");
    expect(bodySprite(SnakeDirection.LEFT, SnakeDirection.UP)).toBe("turnUpLeft");
    expect(bodySprite(SnakeDirection.RIGHT, SnakeDirection.DOWN)).toBe("turnDownRight");
  });
});
