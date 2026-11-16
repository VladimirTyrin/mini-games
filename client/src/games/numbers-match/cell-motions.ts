import type { NumbersMatchPairRemoved, NumbersMatchView } from "@/proto/minigames/v2/numbers_match_pb";

/** How a cell of the current view got where it is through the last action. */
export interface CellMotion {
  /** Rows the cell moved up because rows above it were deleted. */
  readonly rowsShiftedUp: number;
  /** The cell was part of the pair the last action removed. */
  readonly justRemoved: boolean;
  /** Position among the values the last refill appended; `null` for other cells. */
  readonly refillOrder: number | null;
}

const STILL: CellMotion = { rowsShiftedUp: 0, justRemoved: false, refillOrder: null };

/** One motion per cell of `view.cells`, derived from `view.lastAction`. */
export function cellMotions(view: NumbersMatchView): CellMotion[] {
  const motions = view.cells.map(() => STILL);
  const action = view.lastAction?.action;
  if (action?.case === "pairRemoved") applyPairRemoved(motions, action.value, view.columns);
  if (action?.case === "refilled") {
    action.value.values.forEach((_, order) => {
      const index = action.value.startIndex + order;
      if (index < motions.length) motions[index] = { ...STILL, refillOrder: order };
    });
  }
  return motions;
}

function applyPairRemoved(motions: CellMotion[], removal: NumbersMatchPairRemoved, columns: number): void {
  if (columns === 0) return;
  const deletedRows = new Set(removal.deletedRows);
  const rowsShiftedUp = (rowBeforeDeletion: number) =>
    removal.deletedRows.filter((deleted) => deleted < rowBeforeDeletion).length;

  if (deletedRows.size > 0) {
    const shiftByRow = rowShifts(Math.ceil(motions.length / columns), deletedRows);
    motions.forEach((motion, index) => {
      motions[index] = { ...motion, rowsShiftedUp: shiftByRow[Math.floor(index / columns)] ?? 0 };
    });
  }

  const pair = removal.pair;
  for (const indexBeforeDeletion of pair ? [pair.firstIndex, pair.secondIndex] : []) {
    const rowBeforeDeletion = Math.floor(indexBeforeDeletion / columns);
    if (deletedRows.has(rowBeforeDeletion)) continue;
    const index = indexBeforeDeletion - rowsShiftedUp(rowBeforeDeletion) * columns;
    const motion = motions[index];
    if (motion) motions[index] = { ...motion, justRemoved: true };
  }
}

/** For every row of the board after deletion: how many deleted rows were above it. */
function rowShifts(rowCount: number, deletedRows: ReadonlySet<number>): number[] {
  const shifts: number[] = [];
  let deletedAbove = 0;
  for (let rowBeforeDeletion = 0; shifts.length < rowCount; rowBeforeDeletion++) {
    if (deletedRows.has(rowBeforeDeletion)) deletedAbove++;
    else shifts.push(deletedAbove);
  }
  return shifts;
}

/**
 * Changes with every accepted command and only then, so re-keying the grid on it restarts the
 * animations for a new action but not for a re-published identical view.
 */
export function actionFrameKey(view: NumbersMatchView): string {
  const stats = view.stats;
  return `${stats?.pairsRemoved ?? 0}:${stats?.refillsUsed ?? 0}:${stats?.hintsUsed ?? 0}`;
}
