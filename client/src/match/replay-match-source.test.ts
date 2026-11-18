import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { create, toBinary } from "@bufbuild/protobuf";
import { beforeAll, describe, expect, it } from "vitest";

import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { MatchHeaderSchema } from "@/proto/minigames/v2/match_pb";
import { NumbersMatchHintMode } from "@/proto/minigames/v2/numbers_match_pb";
import { initSync, LocalHost } from "@/wasm/pkg/mg_wasm";

import { ReplayMatchSource } from "./replay-match-source";

beforeAll(() => {
  initSync({ module: readFileSync(resolve(__dirname, "../wasm/pkg/mg_wasm_bg.wasm")) });
});

function recordedReplay(): Uint8Array {
  const header = create(MatchHeaderSchema, {
    matchId: "m",
    settings: {
      game: { case: "numbersMatch", value: { refills: 5, hintMode: NumbersMatchHintMode.LIMITED } },
    },
    seats: [{ slot: 0, player: { kind: IdentityKind.GUEST, id: "me", displayName: "Me" } }],
  });
  const host = new LocalHost(toBinary(MatchHeaderSchema, header), 1, 2);
  try {
    host.sealFrame();
    return host.replayFile();
  } finally {
    host.free();
  }
}

describe("ReplayMatchSource", () => {
  it("hands its file to a board that reads ahead in it", async () => {
    const bytes = recordedReplay();

    await expect(new ReplayMatchSource(bytes).replayFile()).resolves.toEqual(bytes);
  });
});
