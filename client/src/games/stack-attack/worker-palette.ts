// Seat colours of hard hats and vests; the info panel shows the same ones next to each name.
const WORKER_COLORS = ["#48bb78", "#4299e1", "#ed64a6", "#ecc94b"] as const;
// The same hues darker, for sleeves: arms stay readable against the vest.
const WORKER_SHADES = ["#2f855a", "#2b6cb0", "#b83280", "#b7791f"] as const;

export function workerColor(slot: number): string {
  return WORKER_COLORS[slot % WORKER_COLORS.length] ?? WORKER_COLORS[0];
}

export function workerShade(slot: number): string {
  return WORKER_SHADES[slot % WORKER_SHADES.length] ?? WORKER_SHADES[0];
}
