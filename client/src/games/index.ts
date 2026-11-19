import { LOCALES, messagesFor } from "@/i18n";

import type { GamePlugin, SettingsCase } from "./game-plugin";
import { numbersMatchPlugin } from "./numbers-match";
import { puzzle2048Plugin } from "./puzzle2048";
import { snakePlugin } from "./snake";
import { stackAttackPlugin } from "./stack-attack";
import { tictactoePlugin } from "./tictactoe";
import { wordHuntPlugin } from "./word-hunt";
import { checkersPlugin } from "./checkers";
import { seaBattlePlugin } from "./sea-battle";
// xtask:imports

/** Every game the UI knows. Order is the order on the home screen. */
export const gamePlugins: readonly GamePlugin[] = [
  snakePlugin,
  tictactoePlugin,
  stackAttackPlugin,
  puzzle2048Plugin,
  numbersMatchPlugin,
  wordHuntPlugin,
  checkersPlugin,
  seaBattlePlugin,
  // xtask:plugins
];

export function pluginById(id: string): GamePlugin | undefined {
  return gamePlugins.find((plugin) => plugin.id === id);
}

export function pluginBySettings(settingsCase: SettingsCase | undefined): GamePlugin | undefined {
  return gamePlugins.find((plugin) => plugin.settingsCase === settingsCase);
}

/**
 * The game a title names in any language the app speaks: a saved replay's file keeps the title
 * in the language it was saved in.
 */
export function pluginByTitle(title: string | null): GamePlugin | undefined {
  return gamePlugins.find((plugin) =>
    LOCALES.some((language) => messagesFor(language).games[plugin.settingsCase].title === title),
  );
}
