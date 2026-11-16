import type { MatchPause } from "@/proto/minigames/v2/match_pb";

/** A seat the match waits for, with its deadlines on the local `performance.now()` clock. */
export interface SeatPause {
  slot: number;
  budgetEndsAt: number;
  dropVoteAt: number;
}

export function seatPauses(pauses: readonly MatchPause[], receivedAt: number): SeatPause[] {
  return pauses.map((pause) => ({
    slot: pause.slot,
    budgetEndsAt: receivedAt + pause.budgetLeftMs,
    dropVoteAt: receivedAt + pause.dropVoteInMs,
  }));
}
