import type { StackAttackView } from "@/proto/minigames/v2/stack_attack_pb";

import type { WalkDirection } from "./worker-input-pacer";

/**
 * Column the worker in `slot` reaches by walking one step, if the latest view says the way is
 * clear (both cells it would occupy are inside the field and free of crates). A crate in the way
 * means a push the rules may refuse, so that step is left to the authority. Used only to start
 * the own worker's walk before the frame confirms it.
 */
export function freeWalkTarget(view: StackAttackView, slot: number, direction: WalkDirection): number | null {
  const feet = view.workers[slot]?.feet;
  if (!feet) return null;
  const target = feet.x + (direction === "left" ? -1 : 1);
  if (target < 0 || target >= view.fieldWidth) return null;
  const blocked = view.boxes.some(
    (box) => box.cell?.x === target && (box.cell.y === feet.y || box.cell.y === feet.y + 1),
  );
  return blocked ? null : target;
}
