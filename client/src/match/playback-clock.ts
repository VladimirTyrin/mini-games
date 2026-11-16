/** How fast the schedule drifts back towards earlier arrivals, in ticks per tick. */
const RELAX_PER_TICK = 0.02;

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
    this.epoch = this.epoch === null ? sample : Math.max(sample, this.epoch - RELAX_PER_TICK * this.tickMs);
    return this.epoch + tick * this.tickMs;
  }

  /** A pause or a rebuilt history: the old schedule no longer applies. */
  reset(): void {
    this.epoch = null;
  }
}
