import { describe, expect, it } from "vitest";

import { allowedCells, canGrowInto, cellIndex, roomForTheRest, shipsLeft, shipsOf } from "./fleet-editor";

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

  it("lets two lone decks wait for the cell that joins them", () => {
    // The whole fleet but one three-decker, which is begun at both ends.
    const almost = filled(
      ...([0, 1, 2, 3].map((x) => [x, 0]) as [number, number][]),
      [0, 2],
      [1, 2],
      [2, 2],
      [5, 0],
      [6, 0],
      [8, 0],
      [9, 0],
      [4, 2],
      [5, 2],
      [0, 4],
      [2, 4],
      [4, 4],
      [6, 4],
    );
    const bothEnds = new Set([...almost, ...filled([0, 9], [2, 9])]);
    expect(canGrowInto(bothEnds, FLEET)).toBe(true);
    expect(allowedCells(bothEnds, FLEET).has(cellIndex({ x: 1, y: 9 }))).toBe(true);
  });

  it("says when the ships not yet started have no room left", () => {
    // Everything but the four-decker, spread so that no straight run of four free cells is left.
    const crowded = filled(
      [1, 4],
      [2, 4],
      [3, 4],
      [7, 6],
      [8, 6],
      [9, 6],
      [8, 1],
      [9, 1],
      [2, 8],
      [3, 8],
      [7, 8],
      [7, 9],
      [4, 1],
      [5, 3],
      [4, 6],
      [0, 8],
    );
    expect(canGrowInto(crowded, FLEET)).toBe(true);
    expect(roomForTheRest(crowded, FLEET)).toBe("full");
    expect(roomForTheRest(filled([0, 0], [1, 0]), FLEET)).toBe("fits");
  });

  it("tells which ships are on the board", () => {
    expect(shipsLeft(filled([0, 0], [1, 0], [5, 5]), FLEET).filter((ship) => ship.isPlaced)).toEqual([
      { length: 2, isPlaced: true },
      { length: 1, isPlaced: true },
    ]);
  });
});
