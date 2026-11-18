import { create } from "@bufbuild/protobuf";
import { describe, expect, it } from "vitest";

import { MatchResultSchema, OutcomeKind } from "@/proto/minigames/v2/match_pb";

import { hasCounted, seriesWithResult } from "./match-series";

const won = (slot: number) => create(MatchResultSchema, { kind: OutcomeKind.WINNER, winnerSlots: [slot] });
const draw = create(MatchResultSchema, { kind: OutcomeKind.DRAW });

describe("seriesWithResult", () => {
  it("adds up wins and draws over matches played again", () => {
    let series = seriesWithResult(null, 2, won(1));
    series = seriesWithResult(series, 2, draw);
    series = seriesWithResult(series, 2, won(1));
    series = seriesWithResult(series, 2, won(0));
    expect(series.wins).toEqual([1, 2]);
    expect(series.draws).toBe(1);
  });

  it("counts a solo or cooperative outcome for nobody", () => {
    const series = seriesWithResult(null, 2, create(MatchResultSchema, { kind: OutcomeKind.VICTORY }));
    expect(hasCounted(series)).toBe(false);
  });
});
