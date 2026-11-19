import type { GamePlugin } from "../game-plugin";
import { t } from "@/i18n";
import { SeaBattleBotKind, SeaBattlePhase } from "@/proto/minigames/v2/sea_battle_pb";

import Board from "./SeaBattleBoard.vue";
import HelpEn from "./SeaBattleHelp.en.vue";
import HelpRu from "./SeaBattleHelp.ru.vue";
import SeaBattleIcon from "./SeaBattleIcon.vue";
import SettingsForm from "./SeaBattleSettings.vue";

function botLabel(kind: SeaBattleBotKind): string {
  switch (kind) {
    case SeaBattleBotKind.EASY:
      return t.games.seaBattle.bots.easy;
    case SeaBattleBotKind.MEDIUM:
      return t.games.seaBattle.bots.medium;
    case SeaBattleBotKind.HARD:
      return t.games.seaBattle.bots.hard;
    default:
      return t.common.bot;
  }
}

export const seaBattlePlugin: GamePlugin = {
  id: "sea_battle",
  settingsCase: "seaBattle",
  get title() {
    return t.games.seaBattle.title;
  },
  icon: "🚢",
  Icon: SeaBattleIcon,
  get tagline() {
    return t.games.seaBattle.tagline;
  },
  SettingsForm,
  Board,
  Help: { en: HelpEn, ru: HelpRu },
  describeSettings: () => [t.games.seaBattle.facts.classicRules],
  botLabel(kind) {
    return kind.game.case === "seaBattle" ? botLabel(kind.game.value) : t.common.bot;
  },
  controls: {
    get keyboard() {
      return t.games.seaBattle.controls.keyboard;
    },
    get touch() {
      return t.games.seaBattle.controls.touch;
    },
  },
  describeEnd(view) {
    if (view.game.case !== "seaBattle") return null;
    switch (view.game.value.phase) {
      case SeaBattlePhase.WON:
        return t.games.seaBattle.endReasons.fleetSunk;
      case SeaBattlePhase.FORFEITED:
        return t.games.seaBattle.endReasons.gaveUpOrLeft;
      default:
        return null;
    }
  },
};
