export interface TileColors {
  readonly background: string;
  readonly text: string;
}

const DARK_TEXT = "#776e65";
const LIGHT_TEXT = "#f9f6f2";

const COLORS_BY_VALUE: Readonly<Record<number, TileColors>> = {
  2: { background: "#eee4da", text: DARK_TEXT },
  4: { background: "#ede0c8", text: DARK_TEXT },
  8: { background: "#f2b179", text: LIGHT_TEXT },
  16: { background: "#f59563", text: LIGHT_TEXT },
  32: { background: "#f67c5f", text: LIGHT_TEXT },
  64: { background: "#f65e3b", text: LIGHT_TEXT },
  128: { background: "#edcf72", text: LIGHT_TEXT },
  256: { background: "#edcc61", text: LIGHT_TEXT },
  512: { background: "#edc850", text: LIGHT_TEXT },
  1024: { background: "#edc53f", text: LIGHT_TEXT },
  2048: { background: "#edc22e", text: LIGHT_TEXT },
};

const BEYOND_PALETTE: TileColors = { background: "#3c3a32", text: LIGHT_TEXT };

export function tileColors(value: number): TileColors {
  return COLORS_BY_VALUE[value] ?? BEYOND_PALETTE;
}

/** Font size as a fraction of the cell size, so long numbers still fit their tile. */
export function tileFontScale(value: number): number {
  const digits = String(value).length;
  if (digits <= 2) return 0.45;
  if (digits === 3) return 0.37;
  if (digits === 4) return 0.29;
  return 0.23;
}
