import { create } from "@bufbuild/protobuf";
import { beforeEach, describe, expect, it, vi } from "vitest";

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
      return {};
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
});
