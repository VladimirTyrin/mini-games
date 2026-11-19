import { t } from "@/i18n";

type SnakeSpeedKey = keyof typeof t.games.snake.speeds;

/** Named snake speeds, shared by the settings form and the lobby summary. */
export const SNAKE_SPEEDS: readonly { key: SnakeSpeedKey; tickIntervalMs: number }[] = [
  { key: "slow", tickIntervalMs: 300 },
  { key: "normal", tickIntervalMs: 200 },
  { key: "fast", tickIntervalMs: 120 },
];

/** "Normal", or "250 ms per step" for a custom speed. */
export function snakeSpeedName(tickIntervalMs: number): string {
  const speed = SNAKE_SPEEDS.find((candidate) => candidate.tickIntervalMs === tickIntervalMs);
  return speed ? t.games.snake.speeds[speed.key] : t.games.snake.customSpeed(tickIntervalMs);
}
