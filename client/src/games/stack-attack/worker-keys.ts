import type { WorkerAction } from "./worker-input-pacer";

const ACTION_BY_KEY: ReadonlyMap<string, WorkerAction> = new Map([
  ["ArrowLeft", "left"],
  ["a", "left"],
  ["ArrowRight", "right"],
  ["d", "right"],
  ["ArrowUp", "jump"],
  ["w", "jump"],
  [" ", "jump"],
]);

/** Arrow keys, A/D/W (either case, so Caps Lock does not break it) and Space; `null` for other keys. */
export function workerActionFromKey(key: string): WorkerAction | null {
  return ACTION_BY_KEY.get(key.length === 1 ? key.toLowerCase() : key) ?? null;
}
