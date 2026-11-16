import { describe, expect, it } from "vitest";

import { stepWalkProgress, stepX, stepY, type WorkerStep } from "./worker-motion";

const step = (from: [number, number], to: [number, number], continuesFall = false): WorkerStep => ({
  from: { x: from[0], y: from[1] },
  to: { x: to[0], y: to[1] },
  continuesFall,
});

describe("worker motion", () => {
  it("starts and ends every step on its cells", () => {
    for (const subject of [
      step([2, 0], [3, 0]),
      step([2, 0], [2, 1]),
      step([2, 3], [3, 2]),
      step([1, 2], [1, 1]),
    ]) {
      expect(stepX(subject, 0)).toBeCloseTo(subject.from.x);
      expect(stepY(subject, 0)).toBeCloseTo(subject.from.y);
      expect(stepX(subject, 1)).toBeCloseTo(subject.to.x);
      expect(stepY(subject, 1)).toBeCloseTo(subject.to.y);
    }
  });

  it("eases a walk out: most of the step is covered early", () => {
    expect(stepX(step([2, 0], [3, 0]), 0.5)).toBeGreaterThan(2.6);
  });

  it("reaches the jump apex early and hangs there", () => {
    const jump = step([2, 0], [2, 1]);
    expect(stepY(jump, 0.35)).toBeGreaterThan(0.8);
    expect(stepY(jump, 0.7)).toBeCloseTo(1);
    expect(stepY(jump, 0.9)).toBeCloseTo(1);
  });

  it("accelerates a fall from rest but keeps the speed of a fall that continues", () => {
    expect(stepY(step([2, 3], [2, 2]), 0.5)).toBeCloseTo(2.75);
    expect(stepY(step([2, 3], [2, 2], true), 0.5)).toBeCloseTo(2.5);
  });

  it("steps off a ledge across first, then drops", () => {
    const ledge = step([2, 3], [3, 2]);
    // Across is done after 60% of the tick, the drop only starts after 40%.
    expect(stepX(ledge, 0.6)).toBeCloseTo(3);
    expect(stepY(ledge, 0.4)).toBeCloseTo(3);
    expect(stepY(ledge, 0.6)).toBeLessThan(3);
    expect(stepWalkProgress(ledge, 0.3)).toBeCloseTo(0.5);
  });
});
