import { describe, expect, it } from "vitest";

import { headHitProgress } from "./head-hit";
import { WORKER_DRAWN_HEIGHT_CELLS } from "./worker-dimensions";
import { stepY, type WorkerStep } from "./worker-motion";

// Feet rise from row 0 to row 1; the head cell moves from row 1 to row 2.
const jump: WorkerStep = { from: { x: 3, y: 0 }, to: { x: 3, y: 1 }, continuesFall: false };

describe("headHitProgress", () => {
  it("hits a crate right above the head almost at once", () => {
    expect(headHitProgress(2, jump)).toBeLessThan(0.1);
  });

  it("meets a crate two cells up while the worker is still rising", () => {
    const progress = headHitProgress(3, jump);
    expect(progress).toBeGreaterThan(0.2);
    expect(progress).toBeLessThan(0.7);
    // At that moment the crate's bottom edge touches the hard hat.
    expect(3 - progress).toBeCloseTo(stepY(jump, progress) + WORKER_DRAWN_HEIGHT_CELLS, 1);
  });

  it("hits later the higher the crate starts", () => {
    expect(headHitProgress(2, jump)).toBeLessThan(headHitProgress(3, jump));
  });
});
