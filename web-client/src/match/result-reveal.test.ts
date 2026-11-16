import { describe, expect, it } from "vitest";

import { REAL_TIME_RESULT_DELAY_MS, resultRevealAt, TURN_BASED_RESULT_DELAY_MS } from "./result-reveal";

describe("resultRevealAt", () => {
  it("waits for the last tick's motion to finish, then lets the real-time ending play out", () => {
    // The final frame is scheduled to show at 1_050 and animates for one 200 ms tick.
    expect(resultRevealAt(1_050, 200, 1_000)).toBe(1_250 + REAL_TIME_RESULT_DELAY_MS);
  });

  it("never reveals before the match ended", () => {
    expect(resultRevealAt(100, 200, 5_000)).toBe(5_000 + REAL_TIME_RESULT_DELAY_MS);
  });

  it("gives turn-based games a shorter pause after the final move", () => {
    expect(resultRevealAt(2_000, 0, 2_000)).toBe(2_000 + TURN_BASED_RESULT_DELAY_MS);
  });

  it("counts from the end when no view was shown", () => {
    expect(resultRevealAt(null, 200, 300)).toBe(300 + 200 + REAL_TIME_RESULT_DELAY_MS);
  });
});
