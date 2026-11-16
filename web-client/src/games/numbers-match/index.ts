import type { GamePlugin } from "../game-plugin";

import { hintModeLabel } from "./hint-mode-options";
import Board from "./NumbersMatchBoard.vue";
import SettingsForm from "./NumbersMatchSettings.vue";

export const numbersMatchPlugin: GamePlugin = {
  id: "numbers_match",
  settingsCase: "numbersMatch",
  title: "Numbers Match",
  icon: "🔟",
  tagline: "Pair equal numbers or tens",
  SettingsForm,
  Board,
  describeSettings(settings) {
    if (settings.game.case !== "numbersMatch") return [];
    const value = settings.game.value;
    return [`hints: ${hintModeLabel(value.hintMode).toLowerCase()}`, `${value.refills} refills`];
  },
  botLabel: () => "Bot",
  controlsHint: "Tap two equal numbers or two that sum to 10 · H for a hint · R to refill",
};
