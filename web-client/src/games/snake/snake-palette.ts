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

/** Darker: each channel times `factor` (0..1). */
export function shade(color: RgbColor, factor: number): RgbColor {
  return {
    red: Math.round(color.red * factor),
    green: Math.round(color.green * factor),
    blue: Math.round(color.blue * factor),
  };
}

/** Lighter: `amount` (0..1) of the way to white. */
export function tint(color: RgbColor, amount: number): RgbColor {
  const towardsWhite = (channel: number) => Math.round(channel + (255 - channel) * amount);
  return { red: towardsWhite(color.red), green: towardsWhite(color.green), blue: towardsWhite(color.blue) };
}
