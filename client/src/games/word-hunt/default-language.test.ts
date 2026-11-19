import { create } from "@bufbuild/protobuf";
import { afterEach, describe, expect, it } from "vitest";

import { locale } from "@/i18n";
import { type GameSettings, GameSettingsSchema } from "@/proto/minigames/v2/game_pb";
import { WordHuntLanguage } from "@/proto/minigames/v2/word_hunt_pb";

import { wordHuntPlugin } from "./index";

const coreDefaults = () =>
  create(GameSettingsSchema, { game: { case: "wordHunt", value: { language: WordHuntLanguage.ENGLISH } } });

function languageOf(settings: GameSettings | undefined): WordHuntLanguage | undefined {
  return settings?.game.case === "wordHunt" ? settings.game.value.language : undefined;
}

describe("Word Hunt's default word list", () => {
  afterEach(() => (locale.value = "en"));

  it("is in the interface's language", () => {
    locale.value = "ru";
    expect(languageOf(wordHuntPlugin.defaultSettings?.(coreDefaults()))).toBe(WordHuntLanguage.RUSSIAN);
    locale.value = "en";
    expect(languageOf(wordHuntPlugin.defaultSettings?.(coreDefaults()))).toBe(WordHuntLanguage.ENGLISH);
  });
});
