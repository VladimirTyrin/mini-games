import type { GamePlugin } from "../game-plugin";

import Board from "./StackAttackBoard.vue";
import SettingsForm from "./StackAttackSettings.vue";
import { describeStackAttackEnd } from "./end-reason";

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
  controls: {
    keyboard:
      "Hold ←/→ or A/D to walk and push crates · Space, W or ↑ to jump · jump, then walk to climb a crate",
    touch: "Hold ◀ ▶ to walk and push crates · Jump, then walk to climb a crate",
  },
  describeEnd: describeStackAttackEnd,
};
