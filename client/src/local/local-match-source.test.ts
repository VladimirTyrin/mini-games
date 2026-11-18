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
  onerror: ((event: { message: string; preventDefault(): void }) => void) | null = null;
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

    match.setPaused("hidden", true);
    match.setPaused("hidden", false, 3_000);
    vi.advanceTimersByTime(2_999);
    expect(worker.posted).toEqual(["pause"]);
    vi.advanceTimersByTime(1);

    expect(worker.posted).toEqual(["pause", "resume"]);
    expect(countdowns).toEqual([3_000]);
  });

  it("stays paused by the player when the app comes back from the background", () => {
    const { match, worker } = source();

    match.setPaused("player", true);
    match.setPaused("hidden", true);
    match.setPaused("hidden", false, 3_000);
    vi.advanceTimersByTime(3_000);
    expect(worker.posted).toEqual(["pause"]);

    match.setPaused("player", false, 3_000);
    vi.advanceTimersByTime(3_000);
    expect(worker.posted).toEqual(["pause", "resume"]);
  });

  it("takes no commands while paused or counting down to the restart", async () => {
    const { match, worker } = source();
    match.start();
    const turn = {
      game: { case: "snake", value: { command: { case: "turn", value: { direction: 1 } } } },
    } as const;

    match.setPaused("player", true);
    await expect(match.submit(turn)).rejects.toThrow("the game is paused");
    match.setPaused("player", false, 3_000);
    await expect(match.submit(turn)).rejects.toThrow("the game is paused");
    vi.advanceTimersByTime(3_000);
    void match.submit(turn);

    expect(worker.posted).toEqual(["start", "pause", "resume", "submit"]);
  });

  it("announces a resume without a countdown, so the board stops holding its input", () => {
    const { match } = source();
    const countdowns: number[] = [];
    match.on("resumed", ({ countdownMs }) => countdowns.push(countdownMs));

    match.setPaused("player", true);
    match.setPaused("player", false);

    expect(countdowns).toEqual([0]);
  });

  it("watching bots, takes no commands and passes on the seats' secrets it is shown", async () => {
    const match = new LocalMatchSource(
      create(MatchHeaderSchema, { matchId: "local", seats: [{}, {}] }),
      null,
    );
    const worker = FakeWorker.last;
    if (!worker) throw new Error("no worker");
    const shown: number[] = [];
    match.on("spectatorSecret", ({ slot }) => shown.push(slot));
    match.start();

    await expect(match.submit({ game: { case: "snake", value: {} } })).rejects.toThrow("you are watching");
    worker.reply({ type: "spectatorSecret", slot: 1, secret: new Uint8Array() });

    expect(worker.posted).toEqual(["start"]);
    expect(shown).toEqual([1]);
  });

  it("settles what waits on a worker that broke down", async () => {
    const { match, worker } = source();
    const ended: unknown[] = [];
    match.on("ended", (result) => ended.push(result));
    const replay = match.replayFile();

    worker.onerror?.({ message: "worker script failed", preventDefault: () => undefined });

    await expect(replay).rejects.toThrow("worker script failed");
    expect(ended).toEqual([null]);
  });

  it("ends the match as aborted when the authority fails, instead of leaving an empty board", () => {
    const { match, worker } = source();
    const ended: unknown[] = [];
    match.on("ended", (result) => ended.push(result));

    worker.reply({ type: "failed", message: "wasm did not load" });

    expect(ended).toEqual([null]);
  });
});
