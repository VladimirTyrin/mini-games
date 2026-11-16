import type { GamePlugin } from "../game-plugin";

import { hintModeLabel } from "./hint-mode-options";
import Board from "./NumbersMatchBoard.vue";
import SettingsForm from "./NumbersMatchSettings.vue";
import NumbersMatchIcon from "./NumbersMatchIcon.vue";

export const numbersMatchPlugin: GamePlugin = {
  id: "numbers_match",
  settingsCase: "numbersMatch",
  title: "Numbers Match",
  icon: "🔟",
  Icon: NumbersMatchIcon,
  tagline: "Pair equal numbers or tens",
  SettingsForm,
  Board,
  describeSettings(settings) {
    if (settings.game.case !== "numbersMatch") return [];
    const value = settings.game.value;
    return [`hints: ${hintModeLabel(value.hintMode).toLowerCase()}`, `${value.refills} refills`];
  },
  botLabel: () => "Bot",
  controls: {
    keyboard: "Click two equal numbers or two that sum to 10 · H for a hint · R to refill",
    touch: "Tap two equal numbers or two that sum to 10 · Hint and Refill are below the board",
  },
};
