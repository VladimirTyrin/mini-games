import type { GamePlugin } from "../game-plugin";
import { TicTacToeBotKind, TicTacToeFirstPlayerMode } from "@/proto/minigames/v2/tictactoe_pb";

import { narrowScreenSettings } from "./board-presets";
import Board from "./TicTacToeBoard.vue";
import SettingsForm from "./TicTacToeSettings.vue";
import TicTacToeIcon from "./TicTacToeIcon.vue";

function firstMoveLabel(mode: TicTacToeFirstPlayerMode): string {
  switch (mode) {
    case TicTacToeFirstPlayerMode.HOST:
      return "host starts";
    case TicTacToeFirstPlayerMode.GUEST:
      return "guest starts";
    default:
      return "random start";
  }
}

export const tictactoePlugin: GamePlugin = {
  id: "tictactoe",
  settingsCase: "tictactoe",
  title: "Tic-Tac-Toe",
  icon: "❌",
  Icon: TicTacToeIcon,
  tagline: "N in a row on a big board",
  SettingsForm,
  Board,
  describeSettings(settings) {
    if (settings.game.case !== "tictactoe") return [];
    const value = settings.game.value;
    return [
      `${value.fieldWidth}×${value.fieldHeight}`,
      `${value.winCount} in a row`,
      firstMoveLabel(value.firstPlayer),
    ];
  },
  botLabel(kind) {
    if (kind.game.case !== "tictactoe") return "Bot";
    return kind.game.value === TicTacToeBotKind.MINIMAX ? "Minimax bot" : "Random bot";
  },
  controls: {
    keyboard: "Click a cell to place your mark",
    touch: "Tap a cell to place your mark",
  },
  narrowScreenSettings,
};
