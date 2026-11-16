import { create } from "@bufbuild/protobuf";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MatchHeaderSchema } from "@/proto/minigames/v2/match_pb";

import { LocalMatchSource } from "./local-match-source";
import type { FromWorker, ToWorker } from "./worker-protocol";

/** Records what the source posts; the test plays the worker. */
class FakeWorker {
  static last: FakeWorker | null = null;
  readonly posted: ToWorker["type"][] = [];
  onmessage: ((event: { data: FromWorker }) => void) | null = null;
  constructor() {
    FakeWorker.last = this;
  }
  postMessage(message: ToWorker): void {
    this.posted.push(message.type);
  }
  reply(message: FromWorker): void {
    this.onmessage?.({ data: message });
  }
}

function source(): { match: LocalMatchSource; worker: FakeWorker } {
  const match = new LocalMatchSource(create(MatchHeaderSchema, { matchId: "local" }), 0);
  const worker = FakeWorker.last;
  if (!worker) throw new Error("no worker");
  return { match, worker };
}

describe("LocalMatchSource", () => {
  beforeEach(() => {
    vi.stubGlobal("Worker", FakeWorker);
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("resumes a real-time game only after the countdown it announces", () => {
    const { match, worker } = source();
    const countdowns: number[] = [];
    match.on("resumed", ({ countdownMs }) => countdowns.push(countdownMs));

    match.setPaused(true);
    match.setPaused(false, 3_000);
    vi.advanceTimersByTime(2_999);
    expect(worker.posted).toEqual(["pause"]);
    vi.advanceTimersByTime(1);

    expect(worker.posted).toEqual(["pause", "resume"]);
    expect(countdowns).toEqual([3_000]);
  });

  it("ends the match as aborted when the authority fails, instead of leaving an empty board", () => {
    const { match, worker } = source();
    const ended: unknown[] = [];
    match.on("ended", (result) => ended.push(result));

    worker.reply({ type: "failed", message: "wasm did not load" });

    expect(ended).toEqual([null]);
  });
});
