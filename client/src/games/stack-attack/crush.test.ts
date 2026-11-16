import { describe, expect, it } from "vitest";

import { CRUSHED_SCALE_Y, crushedScaleY, settledCrushBoxY, squashedScaleX } from "./crush";
import { WORKER_DRAWN_HEIGHT_CELLS } from "./worker-dimensions";

describe("crush", () => {
  it("leaves the worker whole until the box touches the hard hat", () => {
    expect(crushedScaleY(5, 3)).toBe(1);
    expect(crushedScaleY(3 + WORKER_DRAWN_HEIGHT_CELLS, 3)).toBe(1);
  });

  it("presses the worker down with the box, never flatter than a fifth", () => {
    expect(crushedScaleY(3 + WORKER_DRAWN_HEIGHT_CELLS / 2, 3)).toBeCloseTo(0.5);
    expect(crushedScaleY(3, 3)).toBeCloseTo(CRUSHED_SCALE_Y);
    expect(crushedScaleY(2, 3)).toBeCloseTo(CRUSHED_SCALE_Y);
  });

  it("widens the squashed worker to 1.4× at full squash", () => {
    expect(squashedScaleX(1)).toBe(1);
    expect(squashedScaleX(0.2)).toBeCloseTo(1.4);
  });

  it("lets the box that ended the game sink onto the flattened worker", () => {
    // The rules stop the box in the head cell (feet + 1).
    expect(settledCrushBoxY(4, 3, 0)).toBe(4);
    const rest = settledCrushBoxY(4, 3, 1);
    expect(rest).toBeLessThan(3.5);
    expect(crushedScaleY(rest, 3)).toBeCloseTo(CRUSHED_SCALE_Y);
  });

  it("does not lift a box that is already lower", () => {
    expect(settledCrushBoxY(3, 3, 1)).toBe(3);
  });
});
