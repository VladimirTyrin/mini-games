import type { GamePlugin, SettingsCase } from "./game-plugin";
import { numbersMatchPlugin } from "./numbers-match";
import { puzzle2048Plugin } from "./puzzle2048";
import { snakePlugin } from "./snake";
import { stackAttackPlugin } from "./stack-attack";
import { tictactoePlugin } from "./tictactoe";
// xtask:imports

/** Every game the UI knows. Order is the order on the home screen. */
export const gamePlugins: readonly GamePlugin[] = [
  snakePlugin,
  tictactoePlugin,
  stackAttackPlugin,
  puzzle2048Plugin,
  numbersMatchPlugin,
  // xtask:plugins
];

export function pluginById(id: string): GamePlugin | undefined {
  return gamePlugins.find((plugin) => plugin.id === id);
}

export function pluginBySettings(settingsCase: SettingsCase | undefined): GamePlugin | undefined {
  return gamePlugins.find((plugin) => plugin.settingsCase === settingsCase);
}
