import type { GamePlugin } from "../game-plugin";

import Board from "./CheckersBoard.vue";
import SettingsForm from "./CheckersSettings.vue";

export const checkersPlugin: GamePlugin = {
  id: "checkers",
  settingsCase: "checkers",
  title: "Checkers",
  icon: "🎲",
  tagline: "Reach the target first",
  SettingsForm,
  Board,
  describeSettings(settings) {
    return settings.game.case === "checkers" ? [`target ${settings.game.value.target}`] : [];
  },
  botLabel: () => "Bot",
  controls: { keyboard: "Press 1, 2 or 3", touch: "Tap +1, +2 or +3" },
};
