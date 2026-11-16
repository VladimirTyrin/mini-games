import { create } from "@bufbuild/protobuf";
import { describe, expect, it } from "vitest";

import { MatchHeaderSchema } from "@/proto/minigames/v2/match_pb";

import {
  buildReplayFileName,
  parseReplayFileName,
  replayFileNameForMatch,
  replayMatchTag,
} from "./replay-file-name";

const startedAt = new Date(2026, 10, 16, 14, 5, 33);

describe("buildReplayFileName", () => {
  it("names the game, the local start time and the match", () => {
    expect(buildReplayFileName({ gameTitle: "Snake", startedAt, matchTag: "3F9A-1c2d-77" })).toBe(
      "Snake 2026-11-16 14-05-33 3f9a1c.mgreplay",
    );
  });

  it("drops characters file systems refuse and never leaves the title empty", () => {
    expect(buildReplayFileName({ gameTitle: 'a/b:c*"?', startedAt, matchTag: "local" })).toBe(
      "abc 2026-11-16 14-05-33 local.mgreplay",
    );
    expect(buildReplayFileName({ gameTitle: " / ", startedAt, matchTag: "" })).toBe(
      "Replay 2026-11-16 14-05-33 match.mgreplay",
    );
  });

  it("is the same for the same match, so saving twice overwrites", () => {
    const header = create(MatchHeaderSchema, {
      matchId: "0d5e2b7c-aaaa",
      startedAtUnixMs: BigInt(startedAt.getTime()),
    });
    expect(replayFileNameForMatch(header, "Tic-Tac-Toe")).toBe(
      "Tic-Tac-Toe 2026-11-16 14-05-33 0d5e2b.mgreplay",
    );
    expect(replayFileNameForMatch(header, "Tic-Tac-Toe")).toBe(replayFileNameForMatch(header, "Tic-Tac-Toe"));
  });
});

describe("replayMatchTag", () => {
  it("keeps the first letters and digits of the id", () => {
    expect(replayMatchTag("local")).toBe("local");
    expect(replayMatchTag("--AB--12cd34ef")).toBe("ab12cd");
  });
});

describe("parseReplayFileName", () => {
  it("reads back what buildReplayFileName wrote", () => {
    for (const gameTitle of ["Snake", "Stack Attack", "Tic-Tac-Toe", "2048"]) {
      const parts = { gameTitle, startedAt, matchTag: "3f9a1c" };
      expect(parseReplayFileName(buildReplayFileName(parts))).toEqual(parts);
    }
  });

  it.each([
    "mini-games-0d5e2b7c.mgreplay",
    "Snake 2026-11-16 14-05-33 3f9a1c.txt",
    "Snake 2026-02-31 14-05-33 3f9a1c.mgreplay",
    "Snake 2026-11-16 24-05-33 3f9a1c.mgreplay",
    " 2026-11-16 14-05-33 3f9a1c.mgreplay",
  ])("does not recognise %j", (fileName) => {
    expect(parseReplayFileName(fileName)).toBeNull();
  });
});
