import type { ViewSnapshot } from "@/match/match-controller";

/**
 * How far rendering is between the previous and the current view, 0..1. Real-time boards draw
 * one tick behind and lerp positions with this, so motion is smooth at any display refresh rate.
 */
export function tickProgress(current: ViewSnapshot | null, tickMs: number, now: number): number {
  if (!current || tickMs <= 0) return 1;
  return Math.min(1, Math.max(0, (now - current.at) / tickMs));
}

export function lerp(from: number, to: number, progress: number): number {
  return from + (to - from) * progress;
}
