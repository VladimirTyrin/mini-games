import { t } from "@/i18n";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import type { Identity } from "@/proto/minigames/v2/identity_pb";

import { pluginBySettings } from ".";

/** A lobby's name as shown: the host's pick, else one made of the host's name and the game. */
export function lobbyDisplayName(lobby: {
  readonly name: string;
  readonly host?: Identity;
  readonly settings?: GameSettings;
}): string {
  if (lobby.name) return lobby.name;
  const game = pluginBySettings(lobby.settings?.game.case)?.title;
  if (!game) return t.lobby.title;
  return lobby.host ? t.lobby.defaultName(lobby.host.displayName, game) : game;
}
