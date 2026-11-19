import { create } from "@bufbuild/protobuf";
import { afterEach, describe, expect, it } from "vitest";

import { messagesFor, setLocale } from "@/i18n";
import { GameSettingsSchema } from "@/proto/minigames/v2/game_pb";
import { IdentityKind, IdentitySchema } from "@/proto/minigames/v2/identity_pb";

import { pluginByTitle } from ".";
import { lobbyDisplayName } from "./lobby-name";

const host = create(IdentitySchema, { kind: IdentityKind.GUEST, id: "vlad", displayName: "Vlad" });
const snakeSettings = create(GameSettingsSchema, { game: { case: "snake", value: {} } });

describe("lobbyDisplayName", () => {
  afterEach(() => setLocale("en"));

  it("names a lobby without a name after its host and game, in the player's language", () => {
    const lobby = { name: "", host, settings: snakeSettings };
    expect(lobbyDisplayName(lobby)).toBe("Vlad's Snake");
    setLocale("ru");
    expect(lobbyDisplayName(lobby)).toBe(`${messagesFor("ru").games.snake.title} · Vlad`);
  });

  it("shows the host's own name as typed", () => {
    setLocale("ru");
    expect(lobbyDisplayName({ name: "Friday night", host, settings: snakeSettings })).toBe("Friday night");
  });
});

describe("pluginByTitle", () => {
  afterEach(() => setLocale("en"));

  it("finds a game by its title in any language, whatever the current one", () => {
    setLocale("en");
    expect(pluginByTitle(messagesFor("ru").games.seaBattle.title)?.id).toBe("sea_battle");
    expect(pluginByTitle("Sea Battle")?.id).toBe("sea_battle");
    expect(pluginByTitle("Chess")).toBeUndefined();
  });
});
