import { create } from "@bufbuild/protobuf";
import { describe, expect, it } from "vitest";

import {
  StackAttackCraneState,
  type StackAttackView,
  StackAttackViewSchema,
} from "@/proto/minigames/v2/stack_attack_pb";

import { StackAttackTransition, fallFromRest } from "./stack-attack-transition";

const HEIGHT = 8;

function view(craneX: number, state: StackAttackCraneState, dropped: boolean): StackAttackView {
  return create(StackAttackViewSchema, {
    fieldWidth: 10,
    fieldHeight: HEIGHT,
    cranes: [{ id: 1, x: craneX, targetX: 4, state, patternId: 0 }],
    boxes: dropped ? [{ id: 7, cell: { x: 4, y: HEIGHT - 1 }, patternId: 0, falling: true }] : [],
    events: dropped ? [{ event: { case: "boxDropped", value: { craneId: 1, boxId: 7, x: 4 } } }] : [],
  });
}

describe("StackAttackTransition", () => {
  it("starts a dropped box under the crane's previous column, so it rides along with the crane", () => {
    const before = view(3, StackAttackCraneState.MOVING, false);
    const after = view(4, StackAttackCraneState.DROPPING, true);
    const transition = new StackAttackTransition(after, before, 0);
    const box = after.boxes[0];
    if (!box) throw new Error("no box");
    expect(transition.boxStartOf(box)).toEqual({ x: 3, y: HEIGHT });
  });

  it("drops straight down when the crane was already above the column", () => {
    const before = view(4, StackAttackCraneState.MOVING, false);
    const after = view(4, StackAttackCraneState.DROPPING, true);
    const box = after.boxes[0];
    if (!box) throw new Error("no box");
    expect(new StackAttackTransition(after, before, 0).boxStartOf(box)).toEqual({ x: 4, y: HEIGHT });
  });

  it("lets a released box leave the hook from rest and reach full falling speed by the next cell", () => {
    const before = view(4, StackAttackCraneState.MOVING, false);
    const after = view(4, StackAttackCraneState.DROPPING, true);
    const transition = new StackAttackTransition(after, before, 0);

    // Slower than a constant fall early in the tick, at the cell by its end.
    expect(HEIGHT - (transition.boxRowAt(7, 0.25) ?? 0)).toBeLessThan(0.25);
    expect(transition.boxRowAt(7, 1)).toBe(HEIGHT - 1);
    // One cell per tick at the end: the slope matches the constant fall that follows.
    const step = 1e-4;
    expect((fallFromRest(1) - fallFromRest(1 - step)) / step).toBeCloseTo(1, 3);
    expect(fallFromRest(step) / step).toBeCloseTo(0, 3);
  });
});
