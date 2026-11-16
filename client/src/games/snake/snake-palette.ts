export interface RgbColor {
  readonly red: number;
  readonly green: number;
  readonly blue: number;
}

// One colour per seat (snake allows up to 10); the first keeps the classic green snake.
const SEAT_COLORS: readonly RgbColor[] = [
  { red: 74, green: 222, blue: 128 },
  { red: 96, green: 165, blue: 250 },
  { red: 249, green: 115, blue: 22 },
  { red: 232, green: 121, blue: 249 },
  { red: 250, green: 204, blue: 21 },
  { red: 34, green: 211, blue: 238 },
  { red: 248, green: 113, blue: 113 },
  { red: 167, green: 139, blue: 250 },
  { red: 245, green: 245, blue: 245 },
  { red: 180, green: 83, blue: 9 },
];

export const DEAD_SNAKE_COLOR: RgbColor = { red: 128, green: 128, blue: 128 };

export function snakeColor(slot: number): RgbColor {
  return SEAT_COLORS[slot % SEAT_COLORS.length] ?? DEAD_SNAKE_COLOR;
}

export function cssColor(color: RgbColor): string {
  return `rgb(${color.red} ${color.green} ${color.blue})`;
}

// The sheet is drawn in mid greens; brightness scaled by this keeps the outline darker than the
// fill while the fill lands close to the seat colour.
const SPRITE_TINT_GAIN = 1.8;

/** A sheet pixel recoloured for a seat: its brightness times the seat colour, as v1 did. */
export function tintedBySprite(color: RgbColor, sheetPixel: RgbColor): RgbColor {
  const brightness = ((sheetPixel.red + sheetPixel.green + sheetPixel.blue) / 765) * SPRITE_TINT_GAIN;
  return {
    red: Math.min(255, Math.round(color.red * brightness)),
    green: Math.min(255, Math.round(color.green * brightness)),
    blue: Math.min(255, Math.round(color.blue * brightness)),
  };
}
