/** One example board of the help, drawn by `PairExample`; each language's page gives the caption. */
export interface PairExampleBoard {
  readonly cells: readonly number[];
  readonly pair: readonly [number, number];
  readonly allowed: boolean;
}

// Rows of 9 like the real board: a number, 0 for a crossed-out cell.
export const PAIR_EXAMPLES = {
  row: { cells: [5, 3, 7, 1, 9, 4, 2, 6, 8], pair: [1, 2], allowed: true },
  column: { cells: [2, 6, 1, 9, 3, 7, 5, 8, 4, 4, 6, 8, 2, 5, 1, 9, 3, 7], pair: [1, 10], allowed: true },
  diagonal: { cells: [2, 5, 9, 1, 4, 6, 3, 7, 5, 1, 8, 3, 6, 9, 2, 4, 5, 7], pair: [0, 10], allowed: true },
  acrossCrossedOut: { cells: [7, 4, 0, 0, 0, 4, 9, 2, 1], pair: [1, 5], allowed: true },
  rowEndToNext: {
    cells: [1, 5, 9, 2, 7, 3, 8, 2, 6, 4, 1, 3, 7, 9, 2, 5, 8, 1],
    pair: [8, 9],
    allowed: true,
  },
  acrossCrossedOutAndRowEnd: {
    cells: [2, 7, 4, 1, 9, 3, 8, 0, 0, 0, 0, 2, 6, 5, 1, 7, 4, 9],
    pair: [6, 11],
    allowed: true,
  },
  notMatching: { cells: [9, 3, 5, 1, 7, 2, 4, 6, 8], pair: [1, 2], allowed: false },
  notNeighbours: { cells: [2, 1, 6, 9, 4, 3, 7, 5, 8], pair: [1, 3], allowed: false },
} as const satisfies Record<string, PairExampleBoard>;
