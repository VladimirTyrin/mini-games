import type { GamePlugin } from "../game-plugin";

import Board from "./StackAttackBoard.vue";
import SettingsForm from "./StackAttackSettings.vue";

export const stackAttackPlugin: GamePlugin = {
  id: "stack_attack",
  settingsCase: "stackAttack",
  title: "Stack Attack",
  icon: "📦",
  tagline: "Co-op: survive the falling crates",
  SettingsForm,
  Board,
  describeSettings(settings) {
    if (settings.game.case !== "stackAttack") return [];
    const value = settings.game.value;
    return [`${value.fieldWidth}×${value.fieldHeight}`, "co-op"];
  },
  // Stack Attack has no built-in bot kinds.
  botLabel: () => "Bot",
  controlsHint: "←/→ or A/D to move, push crates · Space, W or ↑ to jump · swipe or tap on touch screens",
};
