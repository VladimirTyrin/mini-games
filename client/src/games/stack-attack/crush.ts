import { easeInQuad } from "@/render/easing";

import { WORKER_DRAWN_HEIGHT_CELLS } from "./worker-dimensions";

/** A crushed worker is pressed down to this fraction of its height. */
export const CRUSHED_SCALE_Y = 0.2;
/** How long the box that ended the game takes to press the worker flat after its last tick. */
export const CRUSH_SETTLE_MS = 160;

/**
 * Vertical scale of a worker under a box whose bottom edge is at height `boxBottom` (world heights
 * grow upwards; a box in row y spans y..y+1): the worker gives way from the moment the box touches
 * its hard hat, never flatter than `CRUSHED_SCALE_Y`.
 */
export function crushedScaleY(boxBottom: number, feetY: number): number {
  return Math.min(1, Math.max(CRUSHED_SCALE_Y, (boxBottom - feetY) / WORKER_DRAWN_HEIGHT_CELLS));
}

/** Squashing keeps the volume roughly: 1.4× as wide when pressed to a fifth of the height. */
export function squashedScaleX(scaleY: number): number {
  return 1 + (1 - scaleY) * 0.5;
}

/**
 * The rules stop the crushing box in the worker's head cell and end the game in that tick, so
 * nothing would show the crush. After the last tick the box keeps sinking onto the flattened
 * worker; `settle` is 0..1 through `CRUSH_SETTLE_MS`.
 */
export function settledCrushBoxY(boxY: number, feetY: number, settle: number): number {
  const restingY = Math.min(boxY, feetY + CRUSHED_SCALE_Y * WORKER_DRAWN_HEIGHT_CELLS);
  return boxY + (restingY - boxY) * easeInQuad(settle);
}
