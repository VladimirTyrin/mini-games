import { describe, expect, it } from "vitest";

import { clamp01, easeInQuad, easeOutCubic, easeOutSine, linear, pulse, segment } from "./easing";

describe("easing", () => {
  it.each([linear, easeOutSine, easeOutCubic, easeInQuad])("%o starts at 0 and ends at 1", (curve) => {
    expect(curve(0)).toBe(0);
    expect(curve(1)).toBeCloseTo(1, 10);
  });

  it("clamps progress outside 0..1", () => {
    expect(clamp01(-0.5)).toBe(0);
    expect(clamp01(1.5)).toBe(1);
    expect(easeOutCubic(2)).toBeCloseTo(1, 10);
    expect(easeInQuad(-1)).toBe(0);
  });

  it("ease-out is ahead of linear and ease-in behind it", () => {
    expect(easeOutSine(0.5)).toBeGreaterThan(0.5);
    expect(easeOutCubic(0.5)).toBeGreaterThan(0.5);
    expect(easeInQuad(0.5)).toBeLessThan(0.5);
  });

  it("pulse rises and returns to rest", () => {
    expect(pulse(0)).toBe(0);
    expect(pulse(0.5)).toBeCloseTo(1, 10);
    expect(pulse(1)).toBeCloseTo(0, 10);
  });
});

describe("segment", () => {
  it("maps a sub-interval onto 0..1", () => {
    expect(segment(0.2, 0.4, 1)).toBe(0);
    expect(segment(0.7, 0.4, 1)).toBeCloseTo(0.5, 10);
    expect(segment(1, 0.4, 1)).toBe(1);
    expect(segment(0.3, 0, 0.6)).toBeCloseTo(0.5, 10);
    expect(segment(0.9, 0, 0.6)).toBe(1);
  });

  it("treats an empty interval as a step", () => {
    expect(segment(0.49, 0.5, 0.5)).toBe(0);
    expect(segment(0.5, 0.5, 0.5)).toBe(1);
  });
});
