import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { create } from "@bufbuild/protobuf";
import { beforeAll, describe, expect, it } from "vitest";

import { GameCommandSchema } from "@/proto/minigames/v2/game_pb";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { MatchHeaderSchema } from "@/proto/minigames/v2/match_pb";
import { NumbersMatchHintMode } from "@/proto/minigames/v2/numbers_match_pb";

import { Mirror } from "./core";
import { initSync } from "./pkg/mg_wasm";

/** The real rules, compiled to WASM, as the page runs them. */
beforeAll(() => {
  initSync({ module: readFileSync(resolve(__dirname, "pkg/mg_wasm_bg.wasm")) });
});

function numbersMatch(): Mirror {
  return Mirror.forHeader(
    create(MatchHeaderSchema, {
      matchId: "m",
      settings: {
        game: { case: "numbersMatch", value: { refills: 5, hintMode: NumbersMatchHintMode.LIMITED } },
      },
      seats: [{ slot: 0, player: { kind: IdentityKind.GUEST, id: "me", displayName: "Me" } }],
    }),
  );
}

const refill = create(GameCommandSchema, {
  game: { case: "numbersMatch", value: { command: { case: "refill", value: {} } } },
});

describe("Mirror", () => {
  it("answers in the rules' own words why a command would be refused", () => {
    const mirror = numbersMatch();
    try {
      // Nothing is dealt before the authority's first frame.
      expect(mirror.refusalOf(0, refill)).toBe("the board is not dealt yet");
      expect(mirror.preview(0, refill)).toEqual({ kind: "refused", reason: "the board is not dealt yet" });
    } finally {
      mirror.free();
    }
  });
});
