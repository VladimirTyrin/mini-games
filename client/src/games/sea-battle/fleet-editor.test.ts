import { describe, expect, it } from "vitest";

import { allowedCells, canGrowInto, cellIndex, shipsLeft, shipsOf } from "./fleet-editor";

const FLEET = [4, 3, 3, 2, 2, 2, 1, 1, 1, 1];

function filled(...cells: [number, number][]): Set<number> {
  return new Set(cells.map(([x, y]) => cellIndex({ x, y })));
}

describe("fleet editor", () => {
  it("joins cells sharing a side into one ship", () => {
    expect(shipsOf(filled([2, 3], [3, 3], [4, 3], [7, 7]))).toEqual([
      { x: 2, y: 3, length: 3, horizontal: true },
      { x: 7, y: 7, length: 1, horizontal: true },
    ]);
    expect(shipsOf(filled([5, 1], [5, 2]))).toEqual([{ x: 5, y: 1, length: 2, horizontal: false }]);
  });

  it("splits a ship when a middle cell is cleared", () => {
    expect(shipsOf(filled([0, 0], [1, 0], [3, 0]))).toEqual([
      { x: 0, y: 0, length: 2, horizontal: true },
      { x: 3, y: 0, length: 1, horizontal: true },
    ]);
  });

  it("allows neither a bend, nor a corner touch, nor a ship longer than four", () => {
    const three = filled([0, 0], [1, 0], [2, 0]);
    const allowed = allowedCells(three, FLEET);
    expect(allowed.has(cellIndex({ x: 3, y: 0 }))).toBe(true);
    expect(allowed.has(cellIndex({ x: 0, y: 1 }))).toBe(false);
    expect(allowed.has(cellIndex({ x: 3, y: 1 }))).toBe(false);
    expect(allowed.has(cellIndex({ x: 5, y: 5 }))).toBe(true);
    expect(canGrowInto(filled([0, 0], [1, 0], [2, 0], [3, 0], [4, 0]), FLEET)).toBe(false);
  });

  it("allows no more ships of a length than the fleet has", () => {
    const three = (y: number): [number, number][] => [
      [0, y],
      [1, y],
      [2, y],
    ];
    // Three ships of three may still be the four and the two threes.
    const threeThrees = filled(...three(0), ...three(2), ...three(4));
    expect(canGrowInto(threeThrees, FLEET)).toBe(true);
    // A fourth has no ship left to become.
    expect(canGrowInto(new Set([...threeThrees, ...filled(...three(6))]), FLEET)).toBe(false);
  });

  it("tells which ships are on the board", () => {
    expect(shipsLeft(filled([0, 0], [1, 0], [5, 5]), FLEET).filter((ship) => ship.isPlaced)).toEqual([
      { length: 2, isPlaced: true },
      { length: 1, isPlaced: true },
    ]);
  });
});
