import { create, type MessageInitShape } from "@bufbuild/protobuf";
import { describe, expect, it } from "vitest";

import {
  StackAttackEndReason,
  type StackAttackView,
  StackAttackViewSchema,
  StackAttackWorkerStatus,
} from "@/proto/minigames/v2/stack_attack_pb";

import { StackAttackTransition } from "./stack-attack-transition";
import { WorkerAnimator } from "./worker-animator";
import { WORKER_DRAWN_HEIGHT_CELLS } from "./worker-dimensions";

const TICK_MS = 200;

type ViewInit = Omit<Exclude<MessageInitShape<typeof StackAttackViewSchema>, StackAttackView>, "$typeName">;

function view(init: ViewInit & { feetX?: number }): StackAttackView {
  const { feetX = 2, ...rest } = init;
  return create(StackAttackViewSchema, {
    you: 0,
    fieldWidth: 6,
    fieldHeight: 6,
    tickMs: TICK_MS,
    workers: [{ slot: 0, feet: { x: feetX, y: 0 }, status: StackAttackWorkerStatus.ALIVE }],
    ...rest,
  });
}

describe("WorkerAnimator", () => {
  it("walks the own worker as soon as the move is sent and keeps going when the frame confirms it", () => {
    const animator = new WorkerAnimator(TICK_MS, 0);
    const first = view({ feetX: 2 });
    animator.observe(new StackAttackTransition(first, null, 0), 0, false);

    animator.predictWalk(3, 50);
    const start = new StackAttackTransition(first, null, 0);
    expect(animator.poseAt(0, start, 1, 150)?.x).toBeGreaterThan(3);
    expect(animator.poseAt(0, start, 1, 150)?.facing).toBe(1);

    // The frame with the move arrives at 200 and is shown from 220: no jump back to the old cell.
    const confirmed = new StackAttackTransition(view({ feetX: 3 }), first, 220);
    const before = animator.poseAt(0, start, 1, 200)?.x ?? 0;
    animator.observe(confirmed, 200, false);
    expect(animator.poseAt(0, confirmed, 0, 200)?.x).toBeCloseTo(before);
    expect(animator.poseAt(0, confirmed, 1, 400)?.x).toBeCloseTo(3.5);
  });

  it("walks back smoothly when the authority did not move the worker", () => {
    const animator = new WorkerAnimator(TICK_MS, 0);
    const first = view({ feetX: 2 });
    animator.observe(new StackAttackTransition(first, null, 0), 0, false);
    animator.predictWalk(1, 0);

    const refused = new StackAttackTransition(view({ feetX: 2 }), first, 200);
    animator.observe(refused, 200, false);
    expect(animator.poseAt(0, refused, 0, 200)?.x).toBeCloseTo(1.5);
    expect(animator.poseAt(0, refused, 1, 600)?.x).toBeCloseTo(2.5);
  });

  it("keeps the prediction while the command is still in flight", () => {
    const animator = new WorkerAnimator(TICK_MS, 0);
    const first = view({ feetX: 2 });
    animator.observe(new StackAttackTransition(first, null, 0), 0, false);
    animator.predictWalk(3, 0);
    const late = new StackAttackTransition(view({ feetX: 2 }), first, 200);
    animator.observe(late, 200, true);
    expect(animator.poseAt(0, late, 1, 400)?.x).toBeCloseTo(3.5);
  });

  it("presses the crushed worker flat under the box that ended the match", () => {
    const animator = new WorkerAnimator(TICK_MS, 0);
    const before = view({ boxes: [{ id: 7, cell: { x: 2, y: 2 }, falling: true }] });
    animator.observe(new StackAttackTransition(before, null, 0), 0, false);
    const crushed = view({
      workers: [{ slot: 0, feet: { x: 2, y: 0 }, status: StackAttackWorkerStatus.CRUSHED }],
      boxes: [{ id: 7, cell: { x: 2, y: 1 }, falling: true }],
      events: [{ event: { case: "workerCrushed", value: { slot: 0, boxId: 7 } } }],
      endReason: StackAttackEndReason.WORKERS_CRUSHED,
    });
    const transition = new StackAttackTransition(crushed, before, 1_000);
    animator.observe(transition, 1_000, false);

    expect(animator.poseAt(0, transition, 0, 1_000)?.scaleY).toBeCloseTo(1);
    // At the end of the tick the box sits in the head cell: the worker is pressed to one cell.
    expect(animator.poseAt(0, transition, 1, 1_200)?.scaleY).toBeCloseTo(1 / WORKER_DRAWN_HEIGHT_CELLS);
    const flat = animator.poseAt(0, transition, 1, 2_000);
    expect(flat?.scaleY).toBeCloseTo(0.2);
    expect(flat?.scaleX).toBeCloseTo(1.4);
    expect(flat?.eyes).toBe("dazed");
    // The box rests on the flattened worker instead of stopping at head height.
    expect(animator.boxRowAt(transition, 7, 1, 2_000)).toBeCloseTo(0.2 * WORKER_DRAWN_HEIGHT_CELLS);
  });

  it("faces other workers where they last walked", () => {
    const animator = new WorkerAnimator(TICK_MS, null);
    const first = view({ feetX: 2 });
    animator.observe(new StackAttackTransition(first, null, 0), 0, false);
    const moved = new StackAttackTransition(view({ feetX: 1 }), first, 200);
    animator.observe(moved, 200, false);
    const pose = animator.poseAt(0, moved, 0.5, 300);
    expect(pose?.facing).toBe(-1);
    expect(pose?.walkFrame).toBeGreaterThanOrEqual(0);
    expect(animator.poseAt(0, moved, 1, 400)?.walkFrame).toBe(-1);
  });
});
