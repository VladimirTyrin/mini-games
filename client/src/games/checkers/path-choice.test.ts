import { describe, expect, it } from "vitest";

import { choosePath } from "./path-choice";

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

  it("switches to another piece and deselects the picked one", () => {
    expect(choosePath(simple, [18], 20)).toEqual({ kind: "partial", selection: [20] });
    expect(choosePath(simple, [18], 18)).toEqual({ kind: "partial", selection: [] });
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

  it("lands back on the starting square of a king's capture", () => {
    const loop = [[0, 18, 36, 18, 0]];
    expect(choosePath(loop, [0, 18, 36, 18], 0)).toEqual({
      kind: "complete",
      selection: [0, 18, 36, 18, 0],
    });
  });
});
