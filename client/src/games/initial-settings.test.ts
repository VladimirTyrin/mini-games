import { create } from "@bufbuild/protobuf";
import { describe, expect, it, vi } from "vitest";

import { GameSettingsSchema } from "@/proto/minigames/v2/game_pb";

import type { GamePlugin } from "./game-plugin";
import { initialSettings } from "./initial-settings";

vi.mock("@/wasm/core", () => ({
  defaultSettings: () => create(GameSettingsSchema, { game: { case: "snake", value: { fieldWidth: 15 } } }),
}));

const snake = { id: "snake", settingsCase: "snake" } as GamePlugin;

describe("initialSettings", () => {
  it("starts from the settings remembered for the game", () => {
    const remembered = create(GameSettingsSchema, { game: { case: "snake", value: { fieldWidth: 12 } } });
    expect(initialSettings(snake, remembered)).toBe(remembered);
  });

  it("ignores another game's settings saved under this game's id", () => {
    const foreign = create(GameSettingsSchema, { game: { case: "wordHunt", value: { wordLength: 5 } } });
    expect(initialSettings(snake, foreign).game.case).toBe("snake");
  });
});
