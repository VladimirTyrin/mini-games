import { create } from "@bufbuild/protobuf";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MatchCatchUpSchema, MatchHeaderSchema, TickFrameSchema } from "@/proto/minigames/v2/match_pb";
import { Emitter } from "@/net/emitter";

import { MatchController } from "./match-controller";
import type { MatchSource, MatchSourceEvents } from "./match-source";

/** How the fake core treats what it is given: like this build's rules, or another version's. */
const core = { framesReplay: true, catchUpsReplay: true };

vi.mock("@/wasm/core", () => ({
  Mirror: class {
    nextTick = 0;
    tickMs = 100;
    isFinished = false;
    static forHeader(): unknown {
      return new this();
    }
    applyFrame(frame: { tick: bigint }): void {
      if (!core.framesReplay) throw new Error("hash mismatch");
      this.nextTick = Number(frame.tick) + 1;
    }
    applyCatchUp(catchUp: { nextTick: bigint }): void {
      if (!core.catchUpsReplay) throw new Error("hash mismatch");
      this.nextTick = Number(catchUp.nextTick);
    }
    view(): unknown {
      // Which state the view shows, for tests about when it is shown.
      return { shownTick: this.nextTick };
    }
    result(): null {
      return null;
    }
    free(): void {}
  },
}));

class FakeSource implements MatchSource {
  readonly kind = "online";
  readonly header = create(MatchHeaderSchema, { matchId: "m" });
  readonly mySlot = 0;
  readonly events = new Emitter<MatchSourceEvents>();
  catchUpRequests = 0;
  on<K extends keyof MatchSourceEvents>(event: K, handler: (payload: MatchSourceEvents[K]) => void) {
    return this.events.on(event, handler);
  }
  start(): void {}
  submit(): Promise<void> {
    return Promise.resolve();
  }
  submitSecret(): Promise<void> {
    return Promise.resolve();
  }
  requestCatchUp(): void {
    this.catchUpRequests += 1;
  }
  voteDrop(): Promise<void> {
    return Promise.resolve();
  }
  leave(): void {}
  replayFile(): Promise<Uint8Array | null> {
    return Promise.resolve(null);
  }
  frame(tick: number, hashed = false): void {
    this.events.emit(
      "frame",
      create(TickFrameSchema, { tick: BigInt(tick), stateHash: hashed ? 1n : undefined }),
    );
  }
  catchUp(nextTick: number): void {
    this.events.emit("catchUp", create(MatchCatchUpSchema, { nextTick: BigInt(nextTick) }));
  }
}

describe("MatchController", () => {
  beforeEach(() => {
    core.framesReplay = true;
    core.catchUpsReplay = true;
  });

  it("stops resyncing when the authority's history does not replay here", () => {
    const source = new FakeSource();
    const controller = new MatchController(source);
    core.framesReplay = false;
    core.catchUpsReplay = false;

    source.frame(0, true);
    source.catchUp(1);
    source.frame(1, true);

    expect(controller.incompatible.value).toBe(true);
    expect(source.catchUpRequests).toBe(1);
  });

  it("gives up when every catch-up desyncs again at the next checked frame", () => {
    const source = new FakeSource();
    const controller = new MatchController(source);
    core.framesReplay = false;

    // Each requested catch-up replays, but the next checked frame disagrees again.
    source.frame(0, true);
    while (source.catchUpRequests < 10 && !controller.incompatible.value) {
      source.catchUp(0);
      source.frame(0, true);
    }

    expect(controller.incompatible.value).toBe(true);
    expect(source.catchUpRequests).toBe(3);
  });

  it("keeps resyncing occasional desyncs separated by checked frames", () => {
    const source = new FakeSource();
    const controller = new MatchController(source);

    for (let tick = 0; tick < 10; tick += 2) {
      core.framesReplay = false;
      source.frame(tick, true);
      source.catchUp(tick);
      core.framesReplay = true;
      source.frame(tick, true);
      source.frame(tick + 1);
    }

    expect(controller.incompatible.value).toBe(false);
    expect(source.catchUpRequests).toBe(5);
  });

  describe("on screen", () => {
    beforeEach(() => vi.useFakeTimers());
    afterEach(() => vi.useRealTimers());

    const shownTick = (controller: MatchController) =>
      (controller.current.value?.view as unknown as { shownTick: number } | undefined)?.shownTick;

    it("shows a frame that came early in a burst at its own time, after the move before it", () => {
      const source = new FakeSource();
      const controller = new MatchController(source);
      source.frame(0);
      // Frame 1 is late (timers, network); frame 2 comes right behind it.
      vi.advanceTimersByTime(250);
      source.frame(1);
      vi.advanceTimersByTime(5);
      source.frame(2);

      // The move to frame 1 plays out first instead of being cut short.
      expect(shownTick(controller)).toBe(2);
      vi.advanceTimersByTime(90);
      expect(shownTick(controller)).toBe(2);
      vi.advanceTimersByTime(10);
      expect(shownTick(controller)).toBe(3);
    });
  });
});
