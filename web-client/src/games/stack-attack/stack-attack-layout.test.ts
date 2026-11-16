import { describe, expect, it } from "vitest";

import { CRANE_RAIL_ROWS, StackAttackLayout } from "./stack-attack-layout";

describe("StackAttackLayout", () => {
  const layout = StackAttackLayout.fit({ width: 400, height: 900 }, 10, 12);

  it("fits whole-pixel cells to the narrower dimension and centres the field", () => {
    expect(layout.cellSize).toBe(40);
    expect(layout.originX).toBe(0);
    expect(layout.originY).toBeGreaterThan(0);
  });

  it("puts row 0 on the floor and the carried boxes right above the ceiling", () => {
    expect(layout.cellTop(0) + layout.cellSize).toBeCloseTo(layout.floorY);
    expect(layout.cellTop(12)).toBeCloseTo(layout.originY + CRANE_RAIL_ROWS * layout.cellSize);
    expect(layout.cellTop(12) + layout.cellSize).toBeCloseTo(layout.ceilingY);
  });

  it("interpolates between rows", () => {
    expect(layout.cellTop(2.5)).toBeCloseTo((layout.cellTop(2) + layout.cellTop(3)) / 2);
  });
});
