/** Easing curves map progress 0..1 to 0..1; every curve here is clamped, so callers may overshoot. */

export function clamp01(value: number): number {
  return value <= 0 ? 0 : value >= 1 ? 1 : value;
}

/** Progress of a sub-interval `[start, end]` of a 0..1 timeline: 0 before it, 1 after it. */
export function segment(progress: number, start: number, end: number): number {
  if (end <= start) return progress >= end ? 1 : 0;
  return clamp01((progress - start) / (end - start));
}

export function linear(progress: number): number {
  return clamp01(progress);
}

/** Starts briskly and settles: a step that plants its foot. */
export function easeOutSine(progress: number): number {
  return Math.sin((clamp01(progress) * Math.PI) / 2);
}

/** Decelerates hard: a jump losing speed towards its apex. */
export function easeOutCubic(progress: number): number {
  const remaining = 1 - clamp01(progress);
  return 1 - remaining * remaining * remaining;
}

/** Accelerates from rest: falling under gravity. */
export function easeInQuad(progress: number): number {
  const clamped = clamp01(progress);
  return clamped * clamped;
}

/** 0 → 1 → 0 over the interval: a squash that recovers. */
export function pulse(progress: number): number {
  return Math.sin(clamp01(progress) * Math.PI);
}
