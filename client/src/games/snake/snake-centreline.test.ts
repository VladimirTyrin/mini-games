import { describe, expect, it } from "vitest";

import { sampleCentreline } from "./snake-centreline";

describe("sampleCentreline", () => {
  it("runs head first and measures the distance to the tail", () => {
    const line = sampleCentreline([
      { x: 3.5, y: 0.5 },
      { x: 0.5, y: 0.5 },
    ]);
    expect(line[0]).toMatchObject({ x: 3.5, y: 0.5 });
    expect(line[0]?.fromTail).toBeCloseTo(3);
    expect(line.at(-1)).toMatchObject({ x: 0.5, y: 0.5, fromTail: 0 });
    // Heading towards the head: east.
    expect(line[0]?.angle).toBeCloseTo(0);
  });

  it("rounds a turn with a quarter circle, so the line is shorter than its corner", () => {
    const line = sampleCentreline([
      { x: 2.5, y: 2.5 },
      { x: 2.5, y: 0.5 },
      { x: 0.5, y: 0.5 },
    ]);
    // Two straight cells of 1.5 into a radius-0.5 arc instead of the 2 + 2 corner path.
    const expected = 1.5 + 1.5 + (Math.PI / 2) * 0.5;
    expect(line[0]?.fromTail).toBeCloseTo(expected, 2);
    const corner = line.find((point) => Math.abs(point.x - 2.5) < 1e-6 && Math.abs(point.y - 0.5) < 1e-6);
    expect(corner).toBeUndefined();
  });

  it("is empty for no points", () => {
    expect(sampleCentreline([])).toEqual([]);
  });
});
