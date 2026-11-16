import { describe, expect, it } from "vitest";

import { type ParticleBurst, ParticleField } from "./particles";

const burst: ParticleBurst = {
  x: 1,
  y: 2,
  count: 3,
  speed: 4,
  angle: 0,
  spread: 0,
  size: 0.1,
  lifeMs: 1000,
  color: "#fff",
  at: 500,
};

describe("ParticleField", () => {
  it("is active from the burst until its particles die", () => {
    const field = new ParticleField(8, -10, () => 1);
    expect(field.isActive(0)).toBe(false);
    field.emit(burst);
    expect(field.isActive(400)).toBe(true);
    expect(field.isActive(1_400)).toBe(true);
    expect(field.isActive(1_600)).toBe(false);
  });

  it("follows a ballistic path", () => {
    const field = new ParticleField(8, -10, () => 1);
    field.emit(burst);
    const after = field.positionAt(0, 500);
    // random() = 1: full speed along the angle.
    expect(after.x).toBeCloseTo(1 + 4 * 0.5);
    expect(after.y).toBeCloseTo(2 - 0.5 * 10 * 0.25);
  });

  it("reuses the oldest slots when full", () => {
    const field = new ParticleField(2, 0, () => 1);
    field.emit({ ...burst, count: 2, at: 0 });
    field.emit({ ...burst, count: 1, x: 7, at: 10_000 });
    expect(field.positionAt(0, 0).x).toBe(7);
    expect(field.isActive(10_500)).toBe(true);
  });

  it("clear stops everything", () => {
    const field = new ParticleField(4, 0);
    field.emit(burst);
    field.clear();
    expect(field.isActive(600)).toBe(false);
  });
});
