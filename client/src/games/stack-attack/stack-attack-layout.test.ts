import { describe, expect, it } from "vitest";

import { CRANE_RAIL_ROWS, StackAttackLayout } from "./stack-attack-layout";

describe("StackAttackLayout", () => {
  const layout = StackAttackLayout.fit({ width: 400, height: 900 }, 10, 12);

  it("fits whole-pixel cells to the narrower dimension, centred and standing on the bottom", () => {
    expect(layout.cellSize).toBe(40);
    expect(layout.originX).toBe(0);
    expect(layout.originY).toBeGreaterThan(0);
    expect(layout.bottomY).toBeCloseTo(900, 0);
  });

  it("puts row 0 on the floor and the carried boxes right above the ceiling", () => {
    expect(layout.cellTop(0) + layout.cellSize).toBeCloseTo(layout.floorY);
    expect(layout.cellTop(12)).toBeCloseTo(layout.originY + CRANE_RAIL_ROWS * layout.cellSize);
    expect(layout.cellTop(12) + layout.cellSize).toBeCloseTo(layout.ceilingY);
  });

  it("interpolates between rows", () => {
    expect(layout.cellTop(2.5)).toBeCloseTo((layout.cellTop(2) + layout.cellTop(3)) / 2);
  });

  it("maps world heights with the floor surface at 0", () => {
    expect(layout.pixelY(0)).toBeCloseTo(layout.floorY);
    expect(layout.pixelY(1)).toBeCloseTo(layout.cellTop(0));
    expect(layout.pixelX(1.5)).toBeCloseTo(layout.cellLeft(1) + layout.cellSize / 2);
  });
});
