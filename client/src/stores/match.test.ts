import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useMatchStore } from "./match";

// The real ones need the WASM core; the store only wires them together.
vi.mock("@/match/replay-match-source", () => ({
  ReplayMatchSource: class {
    readonly kind = "replay";
    start(): void {}
    leave(): void {}
  },
}));
vi.mock("@/match/match-controller", () => ({
  MatchController: class {
    constructor(readonly source: unknown) {}
    dispose(): void {}
  },
}));
vi.mock("@/games", () => ({ pluginBySettings: () => undefined }));

describe("match store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("keeps the replay it starts, so the replay controls show", () => {
    const match = useMatchStore();
    match.startReplay(new Uint8Array([1]));
    expect(match.replay).not.toBeNull();
    expect(match.controller?.source).toBe(match.replay);
  });

  it("forgets the replay when the match closes", () => {
    const match = useMatchStore();
    match.startReplay(new Uint8Array([1]));
    match.close();
    expect(match.replay).toBeNull();
  });
});
