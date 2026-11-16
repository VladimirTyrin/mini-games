import type { GamePlugin } from "../game-plugin";

import Board from "./StackAttackBoard.vue";
import SettingsForm from "./StackAttackSettings.vue";

// Placeholder until the real plugin lands.
export const stackAttackPlugin: GamePlugin = {
  id: "stack_attack",
  settingsCase: "stackAttack",
  title: "Stack Attack",
  icon: "📦",
  tagline: "Co-op: survive the falling crates",
  SettingsForm,
  Board,
  describeSettings: () => [],
  botLabel: () => "Bot",
  controlsHint: "",
};
