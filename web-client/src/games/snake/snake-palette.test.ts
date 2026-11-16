import { describe, expect, it } from "vitest";

import { cssColor, DEAD_SNAKE_COLOR, snakeColor } from "./snake-palette";

describe("snakeColor", () => {
  it("gives each of the ten seats its own colour, none of them the dead grey", () => {
    const colors = Array.from({ length: 10 }, (_, slot) => cssColor(snakeColor(slot)));
    expect(new Set(colors).size).toBe(10);
    expect(colors).not.toContain(cssColor(DEAD_SNAKE_COLOR));
  });

  it("is stable per seat", () => {
    expect(snakeColor(3)).toEqual(snakeColor(3));
  });
});
