import { create } from "@bufbuild/protobuf";
import { describe, expect, it } from "vitest";

import { IdentityKind, IdentitySchema } from "@/proto/minigames/v2/identity_pb";
import { LobbySeatSchema } from "@/proto/minigames/v2/lobby_pb";
import { MatchHeaderSchema } from "@/proto/minigames/v2/match_pb";

import { awaySlots } from "./away-seats";

const person = (id: string) => create(IdentitySchema, { kind: IdentityKind.GUEST, id, displayName: id });
const header = create(MatchHeaderSchema, {
  seats: [
    { slot: 0, player: person("host") },
    { slot: 1, player: person("guest") },
    { slot: 2, player: person("third") },
  ],
});
const lobbySeats = [
  create(LobbySeatSchema, { occupant: person("host") }),
  create(LobbySeatSchema, { occupant: person("guest"), connectionLost: true }),
  create(LobbySeatSchema, { occupant: person("third"), connectionLost: true }),
];

describe("awaySlots", () => {
  it("lists the people who lost the connection", () => {
    expect(awaySlots(header, lobbySeats, new Set(), new Set())).toEqual([1, 2]);
  });

  it("leaves out a seat the match waits for (its pause shows it) and a dropped one", () => {
    expect(awaySlots(header, lobbySeats, new Set([1]), new Set())).toEqual([2]);
    expect(awaySlots(header, lobbySeats, new Set(), new Set([2]))).toEqual([1]);
  });
});
