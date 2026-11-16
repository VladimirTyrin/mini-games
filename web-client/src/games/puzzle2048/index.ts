import type { GamePlugin } from "../game-plugin";

import Board from "./Puzzle2048Board.vue";
import SettingsForm from "./Puzzle2048Settings.vue";

export const puzzle2048Plugin: GamePlugin = {
  id: "puzzle2048",
  settingsCase: "puzzle2048",
  title: "2048",
  icon: "🔢",
  tagline: "Slide and merge tiles to reach the target",
  SettingsForm,
  Board,
  describeSettings(settings) {
    if (settings.game.case !== "puzzle2048") return [];
    const value = settings.game.value;
    return [`${value.width}×${value.height}`, `to ${value.target}`];
  },
  botLabel: () => "Bot",
  controlsHint: "Arrow keys or WASD to slide the tiles; swipe on touch screens",
};
