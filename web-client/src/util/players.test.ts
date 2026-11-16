import { create } from "@bufbuild/protobuf";
import { describe, expect, it } from "vitest";

import { type Identity, IdentityKind, IdentitySchema } from "@/proto/minigames/v2/identity_pb";
import { MatchHeaderSchema } from "@/proto/minigames/v2/match_pb";

import { playerLabels, sameIdentity, seatLabels } from "./players";

const player = (id: string, displayName: string): Identity =>
  create(IdentitySchema, { kind: IdentityKind.GUEST, id, displayName });

describe("players", () => {
  it("tells namesakes apart by id", () => {
    expect(sameIdentity(player("u_1", "Alex"), player("u_2", "Alex"))).toBe(false);
    expect(sameIdentity(player("u_1", "Alex"), player("u_1", "Alexander"))).toBe(true);
    expect(sameIdentity(player("u_1", "Alex"), null)).toBe(false);
  });

  it("numbers repeated names from the second player on, ignoring case", () => {
    const labels = playerLabels([
      player("u_1", "Alex"),
      player("u_2", "Bo"),
      player("u_3", "alex"),
      player("u_1", "Alex"),
      undefined,
      player("u_4", "Alex"),
    ]);
    expect([...labels]).toEqual([
      ["u_1", "Alex"],
      ["u_2", "Bo"],
      ["u_3", "alex · 2"],
      ["u_4", "Alex · 3"],
    ]);
  });

  it("labels match seats by slot", () => {
    const header = create(MatchHeaderSchema, {
      seats: [
        { slot: 0, player: player("u_1", "Alex") },
        { slot: 1, player: player("u_2", "Alex") },
        { slot: 2 },
      ],
    });
    expect(seatLabels(header)).toEqual(["Alex", "Alex · 2", "Player 3"]);
  });
});
