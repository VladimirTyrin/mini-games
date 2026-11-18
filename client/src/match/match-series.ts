import { create } from "@bufbuild/protobuf";

import { type MatchSeries, MatchSeriesSchema } from "@/proto/minigames/v2/lobby_pb";
import { type MatchResult, OutcomeKind } from "@/proto/minigames/v2/match_pb";

/**
 * The series of matches played again on this device, with `result` counted: what the server
 * keeps for a lobby (`LobbyState.series`), for local matches. Solo and cooperative outcomes
 * count for nobody.
 */
export function seriesWithResult(
  series: MatchSeries | null,
  seats: number,
  result: MatchResult,
): MatchSeries {
  const wins = series?.wins.slice() ?? Array.from({ length: seats }, () => 0);
  let draws = series?.draws ?? 0;
  if (result.kind === OutcomeKind.WINNER) {
    for (const slot of result.winnerSlots) if (slot < wins.length) wins[slot] = (wins[slot] ?? 0) + 1;
  } else if (result.kind === OutcomeKind.DRAW) {
    draws += 1;
  }
  return create(MatchSeriesSchema, { wins, draws });
}

/** Some match of the series counted for someone: there is a score to show. */
export function hasCounted(series: MatchSeries | null | undefined): series is MatchSeries {
  return !!series && (series.draws > 0 || series.wins.some((wins) => wins > 0));
}
