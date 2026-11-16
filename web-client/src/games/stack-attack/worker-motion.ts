import { easeInQuad, easeOutCubic, easeOutSine, linear, segment } from "@/render/easing";

export interface GridPoint {
  readonly x: number;
  readonly y: number;
}

/** One tick of a worker's movement, as the two views around it show it. */
export interface WorkerStep {
  readonly from: GridPoint;
  readonly to: GridPoint;
  /** The tick before this one also moved the worker down: keep the fall's speed, no new ease-in. */
  readonly continuesFall: boolean;
}

/**
 * The rules move a worker one cell per tick and keep a jump airborne exactly one tick, so a plain
 * lerp draws jumps as triangles and walking off a ledge as a 45° glide. These curves give the
 * same cells some weight: a jump decelerates and hangs at its apex, a fall accelerates, and a
 * step off a ledge first moves across (first 60% of the tick) and then drops (last 60%).
 */
const JUMP_RISE_END = 0.7;
const LEDGE_ACROSS_END = 0.6;
const LEDGE_DROP_START = 0.4;

export function isLedgeStep(step: WorkerStep): boolean {
  return step.to.x !== step.from.x && step.to.y < step.from.y;
}

export function stepX(step: WorkerStep, progress: number): number {
  const dx = step.to.x - step.from.x;
  if (dx === 0) return step.to.x;
  const across = isLedgeStep(step) ? segment(progress, 0, LEDGE_ACROSS_END) : progress;
  return step.from.x + dx * easeOutSine(across);
}

export function stepY(step: WorkerStep, progress: number): number {
  const dy = step.to.y - step.from.y;
  if (dy === 0) return step.to.y;
  if (dy > 0) return step.from.y + dy * easeOutCubic(segment(progress, 0, JUMP_RISE_END));
  const drop = isLedgeStep(step) ? segment(progress, LEDGE_DROP_START, 1) : progress;
  return step.from.y + dy * (step.continuesFall ? linear(drop) : easeInQuad(drop));
}

/** How far through its horizontal movement the step is, 0..1 (drives the walk cycle). */
export function stepWalkProgress(step: WorkerStep, progress: number): number {
  return isLedgeStep(step) ? segment(progress, 0, LEDGE_ACROSS_END) : progress;
}
