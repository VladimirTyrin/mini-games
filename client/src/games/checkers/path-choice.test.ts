import { describe, expect, it } from "vitest";

import { canStopHere, choosePath } from "./path-choice";

describe("choosePath", () => {
  const simple = [
    [18, 27],
    [18, 25],
    [20, 29],
  ];

  it("picks a piece, then completes on a landing", () => {
    expect(choosePath(simple, [], 18)).toEqual({ kind: "partial", selection: [18] });
    expect(choosePath(simple, [18], 27)).toEqual({ kind: "complete", selection: [18, 27] });
  });

  it("switches to another piece, and keeps the picked one on its own tap", () => {
    expect(choosePath(simple, [18], 20)).toEqual({ kind: "partial", selection: [20] });
    expect(choosePath(simple, [18], 18)).toEqual({ kind: "partial", selection: [18] });
  });

  it("ignores squares nothing can reach", () => {
    expect(choosePath(simple, [], 27)).toEqual({ kind: "ignore" });
    expect(choosePath(simple, [18], 29)).toEqual({ kind: "ignore" });
  });

  it("walks a capture landing by landing where routes split", () => {
    const captures = [
      [0, 18, 36],
      [0, 18, 4],
    ];
    expect(choosePath(captures, [0], 18)).toEqual({ kind: "partial", selection: [0, 18] });
    expect(choosePath(captures, [0, 18], 4)).toEqual({ kind: "complete", selection: [0, 18, 4] });
  });

  it("jumps to a later landing that only one route reaches", () => {
    const captures = [
      [0, 18, 36, 54],
      [0, 18, 4],
    ];
    expect(choosePath(captures, [0], 54)).toEqual({ kind: "complete", selection: [0, 18, 36, 54] });
  });

  it("does not guess between routes reaching the same square", () => {
    const captures = [
      [0, 18, 36],
      [0, 22, 36],
    ];
    expect(choosePath(captures, [0], 36)).toEqual({ kind: "ignore" });
  });

  it("keeps the piece picked on a tap even where a capture could end on its square", () => {
    const loop = [[0, 18, 36, 18, 0]];
    expect(choosePath(loop, [0], 0)).toEqual({ kind: "partial", selection: [0] });
  });

  it("lands back on the starting square of a king's capture", () => {
    const loop = [[0, 18, 36, 18, 0]];
    expect(choosePath(loop, [0, 18, 36, 18], 0)).toEqual({
      kind: "complete",
      selection: [0, 18, 36, 18, 0],
    });
  });

  describe("under the house rule, where a capture may stop after any jump", () => {
    const stoppable = [
      [0, 18],
      [0, 18, 36],
    ];

    it("waits on a landing the capture could go on from, and may end there", () => {
      expect(choosePath(stoppable, [0], 18)).toEqual({ kind: "partial", selection: [0, 18] });
      expect(canStopHere(stoppable, [0, 18])).toBe(true);
      expect(choosePath(stoppable, [0, 18], 18)).toEqual({ kind: "complete", selection: [0, 18] });
    });

    it("goes on to the next landing", () => {
      expect(choosePath(stoppable, [0, 18], 36)).toEqual({ kind: "complete", selection: [0, 18, 36] });
    });

    it("offers no stop where the capture must go on", () => {
      expect(canStopHere([[0, 18, 36]], [0, 18])).toBe(false);
    });
  });
});
