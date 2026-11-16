import type { GamePlugin } from "../game-plugin";

import Board from "./Puzzle2048Board.vue";
import SettingsForm from "./Puzzle2048Settings.vue";

// Placeholder until the real plugin lands.
export const puzzle2048Plugin: GamePlugin = {
  id: "puzzle2048",
  settingsCase: "puzzle2048",
  title: "2048",
  icon: "🔢",
  tagline: "Slide and merge tiles",
  SettingsForm,
  Board,
  describeSettings: () => [],
  botLabel: () => "Bot",
  controlsHint: "",
};
