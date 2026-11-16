import {
  type StackAttackBox,
  type StackAttackCrane,
  StackAttackEndReason,
  type StackAttackView,
} from "@/proto/minigames/v2/stack_attack_pb";
import { lerp } from "@/render/tick-progress";

import { type EntityChanges, pairById } from "./pair-by-id";
import type { GridPoint } from "./worker-motion";

/** The worker crush that ended the match: its box keeps sinking after the last tick. */
export interface FinalCrush {
  readonly boxId: number;
  readonly feet: GridPoint;
}

/**
 * Everything the renderer derives from one pair of consecutive views, computed once per frame
 * of the match instead of once per animation frame.
 */
export class StackAttackTransition {
  readonly boxes: EntityChanges<StackAttackBox>;
  readonly cranes: EntityChanges<StackAttackCrane>;
  readonly clearedRows: readonly number[];
  /** Boxes a jumping worker's head broke this tick (they are gone from `view`). */
  readonly brokenBoxIds: ReadonlySet<number>;
  readonly finalCrush: FinalCrush | null;
  /** Where each box present in `view` starts this tick (absent: it does not move). */
  private readonly boxStarts = new Map<number, GridPoint>();
  private readonly boxesById = new Map<number, StackAttackBox>();

  constructor(
    readonly view: StackAttackView,
    readonly previous: StackAttackView | null,
    /** `performance.now()` when showing `view` starts. */
    readonly at: number,
  ) {
    this.boxes = pairById(previous?.boxes ?? [], view.boxes, (box) => box.id);
    this.cranes = pairById(previous?.cranes ?? [], view.cranes, (crane) => crane.id);
    for (const box of view.boxes) this.boxesById.set(box.id, box);
    for (const { before, after } of this.boxes.kept)
      if (before.cell) this.boxStarts.set(after.id, before.cell);
    const clearedRows: number[] = [];
    const brokenBoxIds = new Set<number>();
    let finalCrush: FinalCrush | null = null;
    for (const { event } of view.events) {
      if (event.case === "boxDropped") {
        // A freshly released box falls out of its crane rather than popping up below it. The
        // crane drops it on the tick it reaches the column, so the box starts where the crane
        // was and rides along: starting it in the new column put it a cell ahead of the crane.
        const { boxId, craneId, x } = event.value;
        const crane = this.cranes.kept.find((pair) => pair.after.id === craneId);
        this.boxStarts.set(boxId, { x: crane?.before.x ?? x, y: view.fieldHeight });
      } else if (event.case === "linesCleared") clearedRows.push(...event.value.rows);
      else if (event.case === "boxBroken") brokenBoxIds.add(event.value.boxId);
      else if (event.case === "workerCrushed" && view.endReason === StackAttackEndReason.WORKERS_CRUSHED) {
        const feet = view.workers[event.value.slot]?.feet;
        if (feet) finalCrush = { boxId: event.value.boxId, feet };
      }
    }
    this.clearedRows = clearedRows;
    this.brokenBoxIds = brokenBoxIds;
    this.finalCrush = finalCrush;
  }

  boxStartOf(box: StackAttackBox): GridPoint | undefined {
    return this.boxStarts.get(box.id) ?? box.cell;
  }

  /** Row of the box's bottom edge at `progress` through the tick; `null` if it is not in `view`. */
  boxRowAt(boxId: number, progress: number): number | null {
    const box = this.boxesById.get(boxId);
    if (!box?.cell) return null;
    const start = this.boxStarts.get(boxId) ?? box.cell;
    return lerp(start.y, box.cell.y, progress);
  }
}
