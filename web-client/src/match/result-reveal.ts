/** Long enough to see the final crash or crush of a real-time match play out. */
export const REAL_TIME_RESULT_DELAY_MS = 1200;
/** Long enough to see the final move (a winning line, the last merge) land. */
export const TURN_BASED_RESULT_DELAY_MS = 800;

/**
 * When the result may cover the board. The final frame of a real-time match is shown one tick
 * behind (from `lastViewShownAt` for `tickMs`), so the delay counts from the end of that motion,
 * not from when the match ended.
 */
export function resultRevealAt(lastViewShownAt: number | null, tickMs: number, endedAt: number): number {
  const lastMotionEnds = Math.max(endedAt, (lastViewShownAt ?? endedAt) + Math.max(0, tickMs));
  return lastMotionEnds + (tickMs > 0 ? REAL_TIME_RESULT_DELAY_MS : TURN_BASED_RESULT_DELAY_MS);
}
