import type { MatchPause } from "@/proto/minigames/v2/match_pb";

/** A seat the match waits for, with its deadlines on the local `performance.now()` clock. */
export interface SeatPause {
  slot: number;
  /** `null` when the match waits without a limit (turn-based games). */
  budgetEndsAt: number | null;
  dropVoteAt: number;
}

export function seatPauses(pauses: readonly MatchPause[], receivedAt: number): SeatPause[] {
  return pauses.map((pause) => ({
    slot: pause.slot,
    budgetEndsAt: pause.budgetLeftMs === undefined ? null : receivedAt + pause.budgetLeftMs,
    dropVoteAt: receivedAt + pause.dropVoteInMs,
  }));
}
