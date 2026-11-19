import type { GamePlugin } from "../game-plugin";
import { t } from "@/i18n";
import { TicTacToeBotKind, TicTacToeFirstPlayerMode } from "@/proto/minigames/v2/tictactoe_pb";

import { narrowScreenSettings } from "./board-presets";
import Board from "./TicTacToeBoard.vue";
import HelpEn from "./TicTacToeHelp.en.vue";
import HelpRu from "./TicTacToeHelp.ru.vue";
import SettingsForm from "./TicTacToeSettings.vue";
import TicTacToeIcon from "./TicTacToeIcon.vue";

function firstMoveLabel(mode: TicTacToeFirstPlayerMode): string {
  switch (mode) {
    case TicTacToeFirstPlayerMode.HOST:
      return t.games.tictactoe.facts.hostStarts;
    case TicTacToeFirstPlayerMode.GUEST:
      return t.games.tictactoe.facts.guestStarts;
    default:
      return t.games.tictactoe.facts.randomStart;
  }
}

export const tictactoePlugin: GamePlugin = {
  id: "tictactoe",
  settingsCase: "tictactoe",
  get title() {
    return t.games.tictactoe.title;
  },
  icon: "❌",
  Icon: TicTacToeIcon,
  get tagline() {
    return t.games.tictactoe.tagline;
  },
  SettingsForm,
  Board,
  Help: { en: HelpEn, ru: HelpRu },
  describeSettings(settings) {
    if (settings.game.case !== "tictactoe") return [];
    const value = settings.game.value;
    return [
      `${value.fieldWidth}×${value.fieldHeight}`,
      t.games.tictactoe.inARow(value.winCount),
      firstMoveLabel(value.firstPlayer),
    ];
  },
  botLabel(kind) {
    if (kind.game.case !== "tictactoe") return t.common.bot;
    return kind.game.value === TicTacToeBotKind.MINIMAX
      ? t.games.tictactoe.bots.minimax
      : t.games.tictactoe.bots.random;
  },
  controls: {
    get keyboard() {
      return t.games.tictactoe.controls.keyboard;
    },
    get touch() {
      return t.games.tictactoe.controls.touch;
    },
  },
  narrowScreenSettings,
};
