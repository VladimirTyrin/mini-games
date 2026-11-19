import type { GamePlugin } from "../game-plugin";
import { t } from "@/i18n";

import { hintModeLabel } from "./hint-mode-options";
import Board from "./NumbersMatchBoard.vue";
import HelpEn from "./NumbersMatchHelp.en.vue";
import HelpRu from "./NumbersMatchHelp.ru.vue";
import SettingsForm from "./NumbersMatchSettings.vue";
import NumbersMatchIcon from "./NumbersMatchIcon.vue";

export const numbersMatchPlugin: GamePlugin = {
  id: "numbers_match",
  settingsCase: "numbersMatch",
  get title() {
    return t.games.numbersMatch.title;
  },
  icon: "🔟",
  Icon: NumbersMatchIcon,
  get tagline() {
    return t.games.numbersMatch.tagline;
  },
  SettingsForm,
  Board,
  Help: { en: HelpEn, ru: HelpRu },
  opensHelpFirstTime: true,
  describeSettings(settings) {
    if (settings.game.case !== "numbersMatch") return [];
    const value = settings.game.value;
    const facts = t.games.numbersMatch.facts;
    return [facts.hints(hintModeLabel(value.hintMode).toLowerCase()), facts.refills(value.refills)];
  },
  botLabel: () => t.common.bot,
  controls: {
    get keyboard() {
      return t.games.numbersMatch.controls.keyboard;
    },
    get touch() {
      return t.games.numbersMatch.controls.touch;
    },
  },
};
