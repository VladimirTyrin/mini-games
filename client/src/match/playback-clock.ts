/** How fast the schedule drifts back towards earlier arrivals, in ticks per tick. */
const RELAX_PER_TICK = 0.02;
/**
 * The furthest a frame may be shown after it arrived. Jitter stays well within it; after a stall
 * (frames held up for seconds, then all at once) the schedule jumps forward instead of keeping
 * the screen seconds behind the game for minutes while it relaxes.
 */
const MAX_DELAY_MS = 400;

/**
 * When each tick of a real-time match starts to be shown. Frames arrive with jitter (timers,
 * network); animating from each arrival time makes motion stall and then jump. Tick `k` is shown
 * from `epoch + k * tickMs` instead, where `epoch` follows the upper envelope of the arrivals and
 * only slowly relaxes towards earlier ones, so a frame is never shown before it arrives and
 * ordinary jitter never stops the motion.
 */
export class PlaybackClock {
  private epoch: number | null = null;

  constructor(private readonly tickMs: number) {}

  showAt(tick: number, arrivedAt: number): number {
    if (this.tickMs <= 0) return arrivedAt;
    const sample = arrivedAt - tick * this.tickMs;
    const relaxed =
      this.epoch === null ? sample : Math.max(sample, this.epoch - RELAX_PER_TICK * this.tickMs);
    this.epoch = Math.min(relaxed, sample + MAX_DELAY_MS);
    return this.epoch + tick * this.tickMs;
  }

  /** A pause or a rebuilt history: the old schedule no longer applies. */
  reset(): void {
    this.epoch = null;
  }
}
