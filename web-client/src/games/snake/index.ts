import type { GamePlugin } from "../game-plugin";

import Board from "./SnakeBoard.vue";
import SettingsForm from "./SnakeSettings.vue";

// Placeholder until the real plugin lands.
export const snakePlugin: GamePlugin = {
  id: "snake",
  settingsCase: "snake",
  title: "Snake",
  icon: "🐍",
  tagline: "Classic snake, up to 10 players",
  SettingsForm,
  Board,
  describeSettings: () => [],
  botLabel: () => "Bot",
  controlsHint: "",
};
