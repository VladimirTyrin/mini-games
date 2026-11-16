import { describe, expect, it } from "vitest";

import { WalkTween } from "./walk-tween";

describe("WalkTween", () => {
  it("has no target until placed and does not animate the first placement", () => {
    const tween = new WalkTween(200);
    expect(tween.target).toBeNull();
    tween.walkTo(4, 0);
    expect(tween.target).toBe(4);
    expect(tween.valueAt(0)).toBe(4);
    expect(tween.isMovingAt(0)).toBe(false);
  });

  it("walks one cell in one step time", () => {
    const tween = new WalkTween(200);
    tween.place(4);
    tween.walkTo(5, 1_000);
    expect(tween.valueAt(1_000)).toBe(4);
    expect(tween.valueAt(1_100)).toBeGreaterThan(4.5);
    expect(tween.valueAt(1_200)).toBe(5);
    expect(tween.isMovingAt(1_100)).toBe(true);
    expect(tween.isMovingAt(1_200)).toBe(false);
  });

  it("holds its position until a tween scheduled ahead starts", () => {
    const tween = new WalkTween(200);
    tween.place(4);
    tween.walkTo(5, 1_000);
    expect(tween.valueAt(900)).toBe(4);
  });

  it("continues from where it is drawn when retargeted mid-step (no snap)", () => {
    const tween = new WalkTween(200);
    tween.place(4);
    tween.walkTo(5, 0);
    const drawn = tween.valueAt(100);
    tween.walkTo(4, 100);
    expect(tween.valueAt(100)).toBeCloseTo(drawn);
    expect(tween.target).toBe(4);
    expect(tween.valueAt(100 + 200)).toBe(4);
  });

  it("forgets its position on reset", () => {
    const tween = new WalkTween(200);
    tween.place(4);
    tween.reset();
    expect(tween.target).toBeNull();
  });
});
