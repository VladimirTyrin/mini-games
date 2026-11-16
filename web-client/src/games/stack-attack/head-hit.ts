import { WORKER_DRAWN_HEIGHT_CELLS } from "./worker-dimensions";
import { stepY, type WorkerStep } from "./worker-motion";

const SAMPLES = 50;

/**
 * Progress 0..1 through the jump tick at which a crate broken by a head hit meets the hard hat.
 * The rules break it at the start of the tick; drawn, it keeps falling (one cell per tick, its
 * bottom edge at `crateBottomY` when the tick starts) until the rising worker reaches it.
 */
export function headHitProgress(crateBottomY: number, jump: WorkerStep): number {
  for (let sample = 0; sample <= SAMPLES; sample++) {
    const progress = sample / SAMPLES;
    if (crateBottomY - progress <= stepY(jump, progress) + WORKER_DRAWN_HEIGHT_CELLS) return progress;
  }
  return 1;
}
