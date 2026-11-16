import { describe, expect, it } from "vitest";

import { FacingTracker } from "./facing";

describe("FacingTracker", () => {
  it("faces right until a worker moves", () => {
    expect(new FacingTracker().facing(2)).toBe(1);
  });

  it("follows the last horizontal move per worker", () => {
    const facings = new FacingTracker();
    facings.moved(0, -1);
    facings.moved(1, 1);
    expect(facings.facing(0)).toBe(-1);
    expect(facings.facing(1)).toBe(1);
    facings.moved(0, 2);
    expect(facings.facing(0)).toBe(1);
  });

  it("keeps the facing through jumps and falls", () => {
    const facings = new FacingTracker();
    facings.moved(0, -1);
    facings.moved(0, 0);
    expect(facings.facing(0)).toBe(-1);
  });

  it("forgets everything on reset", () => {
    const facings = new FacingTracker();
    facings.moved(0, -1);
    facings.reset();
    expect(facings.facing(0)).toBe(1);
  });
});
