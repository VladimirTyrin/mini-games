import { create } from "@bufbuild/protobuf";
import { describe, expect, it } from "vitest";

import { StackAttackViewSchema, StackAttackWorkerStatus } from "@/proto/minigames/v2/stack_attack_pb";

import { freeWalkTarget } from "./walk-target";

function viewWith(feet: { x: number; y: number }, boxes: { x: number; y: number }[]) {
  return create(StackAttackViewSchema, {
    fieldWidth: 5,
    fieldHeight: 6,
    workers: [{ slot: 0, feet, status: StackAttackWorkerStatus.ALIVE }],
    boxes: boxes.map((cell, id) => ({ id, cell })),
  });
}

describe("freeWalkTarget", () => {
  it("predicts a step into free cells", () => {
    expect(freeWalkTarget(viewWith({ x: 2, y: 0 }, []), 0, "left")).toBe(1);
    expect(freeWalkTarget(viewWith({ x: 2, y: 0 }, []), 0, "right")).toBe(3);
  });

  it("leaves pushes and blocked steps to the authority", () => {
    expect(freeWalkTarget(viewWith({ x: 2, y: 0 }, [{ x: 3, y: 0 }]), 0, "right")).toBeNull();
    expect(freeWalkTarget(viewWith({ x: 2, y: 0 }, [{ x: 1, y: 1 }]), 0, "left")).toBeNull();
  });

  it("ignores crates below the feet or above the head", () => {
    expect(
      freeWalkTarget(
        viewWith({ x: 2, y: 1 }, [
          { x: 3, y: 0 },
          { x: 3, y: 3 },
        ]),
        0,
        "right",
      ),
    ).toBe(3);
  });

  it("stops at the walls", () => {
    expect(freeWalkTarget(viewWith({ x: 0, y: 0 }, []), 0, "left")).toBeNull();
    expect(freeWalkTarget(viewWith({ x: 4, y: 0 }, []), 0, "right")).toBeNull();
  });

  it("knows nothing about seats without a worker", () => {
    expect(freeWalkTarget(viewWith({ x: 2, y: 0 }, []), 3, "left")).toBeNull();
  });
});
