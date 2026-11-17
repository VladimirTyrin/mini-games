import { describe, expect, it } from "vitest";

import { crowdedShips, type EditorShip, placed, rotated, shipCells } from "./fleet-editor";

const four: EditorShip = { x: 0, y: 0, length: 4, horizontal: true };

describe("fleet editor", () => {
  it("keeps a moved ship on the board", () => {
    expect(placed(four, 9, 9)).toEqual({ ...four, x: 6, y: 9 });
    expect(placed(four, -3, 2)).toEqual({ ...four, x: 0, y: 2 });
  });

  it("turns a ship about its bow and shifts it back onto the board", () => {
    expect(rotated({ ...four, x: 2, y: 8 })).toEqual({ x: 2, y: 6, length: 4, horizontal: false });
    expect(shipCells(rotated(four))).toEqual([
      { x: 0, y: 0 },
      { x: 0, y: 1 },
      { x: 0, y: 2 },
      { x: 0, y: 3 },
    ]);
  });

  it("marks ships that touch, even at a corner, and leaves the others", () => {
    const single = (x: number, y: number): EditorShip => ({ x, y, length: 1, horizontal: true });
    expect(crowdedShips([four, single(4, 1), single(6, 6)])).toEqual(new Set([0, 1]));
    expect(crowdedShips([four, single(5, 0)])).toEqual(new Set());
  });
});
