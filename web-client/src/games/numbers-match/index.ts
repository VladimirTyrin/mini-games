import type { GamePlugin } from "../game-plugin";

import Board from "./NumbersMatchBoard.vue";
import SettingsForm from "./NumbersMatchSettings.vue";

// Placeholder until the real plugin lands.
export const numbersMatchPlugin: GamePlugin = {
  id: "numbers_match",
  settingsCase: "numbersMatch",
  title: "Numbers Match",
  icon: "🔟",
  tagline: "Pair equal numbers or tens",
  SettingsForm,
  Board,
  describeSettings: () => [],
  botLabel: () => "Bot",
  controlsHint: "",
};
