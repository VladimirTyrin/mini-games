/** Named snake speeds, shared by the settings form and the lobby summary. */
export const SNAKE_SPEEDS: readonly { key: string; label: string; tickIntervalMs: number }[] = [
  { key: "slow", label: "Slow", tickIntervalMs: 300 },
  { key: "normal", label: "Normal", tickIntervalMs: 200 },
  { key: "fast", label: "Fast", tickIntervalMs: 120 },
];

/** "Normal", or "250 ms per step" for a custom speed. */
export function snakeSpeedName(tickIntervalMs: number): string {
  return (
    SNAKE_SPEEDS.find((speed) => speed.tickIntervalMs === tickIntervalMs)?.label ??
    `${tickIntervalMs} ms per step`
  );
}
