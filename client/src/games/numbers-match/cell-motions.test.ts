import { create, type MessageInitShape } from "@bufbuild/protobuf";
import { describe, expect, it } from "vitest";

import { NumbersMatchCellState, NumbersMatchViewSchema } from "@/proto/minigames/v2/numbers_match_pb";

import { actionFrameKey, cellMotions } from "./cell-motions";

type ViewInit = MessageInitShape<typeof NumbersMatchViewSchema>;

const COLUMNS = 3;

function view(rows: number, lastAction: ViewInit["lastAction"], stats?: ViewInit["stats"]) {
  const cells = Array.from({ length: rows * COLUMNS }, () => ({
    value: 1,
    state: NumbersMatchCellState.ACTIVE,
  }));
  return create(NumbersMatchViewSchema, { rows, columns: COLUMNS, cells, lastAction, stats });
}

const still = { rowsShiftedUp: 0, justRemoved: false, refillOrder: null };

describe("cellMotions", () => {
  it("keeps every cell still before the first action and after a hint", () => {
    expect(cellMotions(view(2, undefined))).toEqual(Array(6).fill(still));
    expect(cellMotions(view(2, { action: { case: "hintShown", value: {} } }))).toEqual(Array(6).fill(still));
  });

  it("marks a removed pair when no row was deleted", () => {
    const motions = cellMotions(
      view(2, { action: { case: "pairRemoved", value: { pair: { firstIndex: 4, secondIndex: 1 } } } }),
    );
    expect(motions.flatMap((motion, index) => (motion.justRemoved ? [index] : []))).toEqual([1, 4]);
    expect(motions.every((motion) => motion.rowsShiftedUp === 0)).toBe(true);
  });

  it("shifts rows below deleted rows up and maps removed cells to their new index", () => {
    // Five rows before deletion; rows 1 and 3 went. Cell 1 is in kept row 0, cell 4 in deleted row 1.
    const motions = cellMotions(
      view(3, {
        action: {
          case: "pairRemoved",
          value: { pair: { firstIndex: 1, secondIndex: 4 }, deletedRows: [1, 3] },
        },
      }),
    );
    expect(motions.map((motion) => motion.rowsShiftedUp)).toEqual([0, 0, 0, 1, 1, 1, 2, 2, 2]);
    expect(motions.flatMap((motion, index) => (motion.justRemoved ? [index] : []))).toEqual([1]);
  });

  it("maps a removed cell below a deleted row to its shifted index", () => {
    const motions = cellMotions(
      view(2, {
        action: {
          case: "pairRemoved",
          value: { pair: { firstIndex: 3, secondIndex: 7 }, deletedRows: [1] },
        },
      }),
    );
    expect(motions.flatMap((motion, index) => (motion.justRemoved ? [index] : []))).toEqual([4]);
    expect(motions[4]).toEqual({ rowsShiftedUp: 1, justRemoved: true, refillOrder: null });
  });

  it("orders appended refill values", () => {
    const motions = cellMotions(
      view(3, { action: { case: "refilled", value: { startIndex: 5, values: [3, 4, 5] } } }),
    );
    expect(motions.map((motion) => motion.refillOrder).slice(4)).toEqual([null, 0, 1, 2, null]);
  });
});

describe("actionFrameKey", () => {
  it("changes with every counted action", () => {
    const before = actionFrameKey(view(1, undefined, { pairsRemoved: 2, refillsUsed: 1, hintsUsed: 0 }));
    const afterHint = actionFrameKey(view(1, undefined, { pairsRemoved: 2, refillsUsed: 1, hintsUsed: 1 }));
    expect(afterHint).not.toBe(before);
    expect(actionFrameKey(view(1, undefined, { pairsRemoved: 2, refillsUsed: 1, hintsUsed: 0 }))).toBe(
      before,
    );
  });
});
