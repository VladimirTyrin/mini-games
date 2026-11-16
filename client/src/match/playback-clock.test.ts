import { describe, expect, it } from "vitest";

import { PlaybackClock } from "./playback-clock";

const TICK_MS = 100;

describe("PlaybackClock", () => {
  it("shows jittered frames at an even pace, never before they arrive", () => {
    const clock = new PlaybackClock(TICK_MS);
    const jitter = [0, 30, 5, 25, 0, 10];
    const shownAt = jitter.map((late, tick) => {
      const arrivedAt = 1_000 + tick * TICK_MS + late;
      const at = clock.showAt(tick, arrivedAt);
      expect(at).toBeGreaterThanOrEqual(arrivedAt);
      return at;
    });
    const gaps = shownAt.slice(1).map((at, index) => at - (shownAt[index] ?? 0));
    // Once the latest arrival is known the schedule keeps a steady pace (it only relaxes slowly).
    for (const gap of gaps.slice(2)) expect(gap).toBeGreaterThanOrEqual(TICK_MS - 2 * 0.02 * TICK_MS);
    for (const gap of gaps.slice(2)) expect(gap).toBeLessThanOrEqual(TICK_MS);
  });

  it("drifts back when frames start arriving earlier", () => {
    const clock = new PlaybackClock(TICK_MS);
    clock.showAt(0, 1_050);
    let lag = 0;
    for (let tick = 1; tick <= 100; tick++) {
      const arrivedAt = 1_000 + tick * TICK_MS;
      lag = clock.showAt(tick, arrivedAt) - arrivedAt;
    }
    expect(lag).toBe(0);
  });

  it("shows turn-based frames when they arrive", () => {
    expect(new PlaybackClock(0).showAt(7, 1_234)).toBe(1_234);
  });
});
