import { create } from "@bufbuild/protobuf";
import { describe, expect, it } from "vitest";

import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { MatchHeaderSchema } from "@/proto/minigames/v2/match_pb";
import { SnakeBotKind } from "@/proto/minigames/v2/snake_pb";

import { matchSeatNames } from "./seat-names";

describe("matchSeatNames", () => {
  it("names bots by their level and their number among the bots, people by their name", () => {
    const bot = (slot: number, kind: SnakeBotKind) => ({
      slot,
      player: { kind: IdentityKind.BUILT_IN_BOT, id: `bot-${slot}`, displayName: `Bot ${slot}` },
      botKind: { game: { case: "snake" as const, value: kind } },
    });
    const header = create(MatchHeaderSchema, {
      settings: { game: { case: "snake", value: {} } },
      seats: [
        { slot: 0, player: { kind: IdentityKind.GUEST, id: "me", displayName: "Alice" } },
        bot(1, SnakeBotKind.EFFICIENT),
        bot(2, SnakeBotKind.CAREFUL),
      ],
    });

    expect(matchSeatNames(header)).toEqual(["Alice", "Efficient bot 1", "Careful bot 2"]);
  });
});
