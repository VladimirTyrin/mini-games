/** Edge length of one sprite in `public/sprites.png`. */
export const SPRITE_PIXELS = 64;

export const APPLE_CELL = { column: 0, row: 3 } as const;
export const HEAD_FACING_RIGHT_CELL = { column: 4, row: 0 } as const;

/**
 * The v1 body tube across its width, from the edge in: outline, side band, centre band. Widths
 * are in cells (the sheet's 64 px straight piece: 50 px wide, 6 px outline, 20 px centre), colours
 * as drawn in the sheet; the renderer recolours them like the sprites.
 */
export const TUBE = {
  width: 50 / 64,
  outlineWidth: 6 / 64,
  centreWidth: 20 / 64,
  outline: { red: 76, green: 112, blue: 75 },
  side: { red: 106, green: 158, blue: 104 },
  centre: { red: 115, green: 182, blue: 113 },
} as const;
