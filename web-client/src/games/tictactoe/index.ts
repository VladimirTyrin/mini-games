import type { GamePlugin } from "../game-plugin";
import { TicTacToeBotKind, TicTacToeFirstPlayerMode } from "@/proto/minigames/v2/tictactoe_pb";

import Board from "./TicTacToeBoard.vue";
import SettingsForm from "./TicTacToeSettings.vue";

export const tictactoePlugin: GamePlugin = {
  id: "tictactoe",
  settingsCase: "tictactoe",
  title: "Tic-Tac-Toe",
  icon: "❌",
  tagline: "N in a row on a big board",
  SettingsForm,
  Board,
  describeSettings(settings) {
    if (settings.game.case !== "tictactoe") return [];
    const value = settings.game.value;
    return [
      `${value.fieldWidth}×${value.fieldHeight}`,
      `${value.winCount} in a row`,
      value.firstPlayer === TicTacToeFirstPlayerMode.HOST ? "host starts" : "random start",
    ];
  },
  botLabel(kind) {
    if (kind.game.case !== "tictactoe") return "Bot";
    return kind.game.value === TicTacToeBotKind.MINIMAX ? "Minimax bot" : "Random bot";
  },
  controlsHint: "Click a cell to place your mark",
};
