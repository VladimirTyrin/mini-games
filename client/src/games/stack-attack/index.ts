import type { GamePlugin } from "../game-plugin";
import { t } from "@/i18n";

import Board from "./StackAttackBoard.vue";
import HelpEn from "./StackAttackHelp.en.vue";
import HelpRu from "./StackAttackHelp.ru.vue";
import SettingsForm from "./StackAttackSettings.vue";
import { describeStackAttackEnd } from "./end-reason";
import StackAttackIcon from "./StackAttackIcon.vue";

export const stackAttackPlugin: GamePlugin = {
  id: "stack_attack",
  settingsCase: "stackAttack",
  get title() {
    return t.games.stackAttack.title;
  },
  icon: "📦",
  Icon: StackAttackIcon,
  get tagline() {
    return t.games.stackAttack.tagline;
  },
  SettingsForm,
  Board,
  Help: { en: HelpEn, ru: HelpRu },
  describeSettings(settings) {
    if (settings.game.case !== "stackAttack") return [];
    const value = settings.game.value;
    return [`${value.fieldWidth}×${value.fieldHeight}`, t.games.stackAttack.facts.coop];
  },
  // Stack Attack has no built-in bot kinds.
  botLabel: () => t.common.bot,
  controls: {
    get keyboard() {
      return t.games.stackAttack.controls.keyboard;
    },
    get touch() {
      return t.games.stackAttack.controls.touch;
    },
  },
  describeEnd: describeStackAttackEnd,
};
