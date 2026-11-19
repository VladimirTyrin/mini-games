import type { GamePlugin } from "../game-plugin";
import { t } from "@/i18n";

import Board from "./Puzzle2048Board.vue";
import HelpEn from "./Puzzle2048Help.en.vue";
import HelpRu from "./Puzzle2048Help.ru.vue";
import SettingsForm from "./Puzzle2048Settings.vue";
import Puzzle2048Icon from "./Puzzle2048Icon.vue";

export const puzzle2048Plugin: GamePlugin = {
  id: "puzzle2048",
  settingsCase: "puzzle2048",
  get title() {
    return t.games.puzzle2048.title;
  },
  icon: "🔢",
  Icon: Puzzle2048Icon,
  get tagline() {
    return t.games.puzzle2048.tagline;
  },
  SettingsForm,
  Board,
  Help: { en: HelpEn, ru: HelpRu },
  describeSettings(settings) {
    if (settings.game.case !== "puzzle2048") return [];
    const value = settings.game.value;
    return [`${value.width}×${value.height}`, t.games.puzzle2048.toTarget(value.target)];
  },
  botLabel: () => t.common.bot,
  controls: {
    get keyboard() {
      return t.games.puzzle2048.controls.keyboard;
    },
    get touch() {
      return t.games.puzzle2048.controls.touch;
    },
  },
};
