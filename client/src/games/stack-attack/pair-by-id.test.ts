import { describe, expect, it } from "vitest";

import { pairById } from "./pair-by-id";

interface Crate {
  id: number;
  x: number;
}

describe("pairById", () => {
  it("splits entities into kept, appeared and vanished", () => {
    const before: Crate[] = [
      { id: 1, x: 0 },
      { id: 2, x: 4 },
    ];
    const after: Crate[] = [
      { id: 2, x: 5 },
      { id: 3, x: 7 },
    ];
    const changes = pairById(before, after, (crate) => crate.id);
    expect(changes.kept).toEqual([{ before: { id: 2, x: 4 }, after: { id: 2, x: 5 } }]);
    expect(changes.appeared).toEqual([{ id: 3, x: 7 }]);
    expect(changes.vanished).toEqual([{ id: 1, x: 0 }]);
  });

  it("treats everything as new without a previous view", () => {
    const changes = pairById([], [{ id: 9, x: 1 }], (crate: Crate) => crate.id);
    expect(changes.kept).toEqual([]);
    expect(changes.appeared).toEqual([{ id: 9, x: 1 }]);
  });
});
