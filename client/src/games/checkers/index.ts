import type { GamePlugin } from "../game-plugin";
import { CheckersBotKind, CheckersEndReason, CheckersWhitePlayer } from "@/proto/minigames/v2/checkers_pb";

import Board from "./CheckersBoard.vue";
import CheckersIcon from "./CheckersIcon.vue";
import SettingsForm from "./CheckersSettings.vue";

function whiteLabel(white: CheckersWhitePlayer): string {
  switch (white) {
    case CheckersWhitePlayer.HOST:
      return "host plays white";
    case CheckersWhitePlayer.GUEST:
      return "guest plays white";
    default:
      return "colours by lot";
  }
}

const BOT_LABELS: Record<number, string> = {
  [CheckersBotKind.EASY]: "Easy bot",
  [CheckersBotKind.MEDIUM]: "Medium bot",
  [CheckersBotKind.HARD]: "Hard bot",
  [CheckersBotKind.MASTER]: "Master bot (slow)",
};

/** Why the game ended, shown with the result. */
export function endReason(reason: CheckersEndReason): string | null {
  switch (reason) {
    case CheckersEndReason.NO_PIECES:
      return "No pieces left";
    case CheckersEndReason.NO_MOVES:
      return "No moves left";
    case CheckersEndReason.OPPONENT_LEFT:
      return "The opponent left";
    case CheckersEndReason.REPETITION:
      return "Draw: the same position three times";
    case CheckersEndReason.KING_MOVES:
      return "Draw: fifteen king moves each";
    default:
      return null;
  }
}

export const checkersPlugin: GamePlugin = {
  id: "checkers",
  settingsCase: "checkers",
  title: "Checkers",
  icon: "⚪",
  Icon: CheckersIcon,
  tagline: "Russian draughts with flying kings",
  SettingsForm,
  Board,
  describeSettings(settings) {
    return settings.game.case === "checkers" ? [whiteLabel(settings.game.value.whitePlayer)] : [];
  },
  botLabel(kind) {
    return kind.game.case === "checkers" ? (BOT_LABELS[kind.game.value] ?? "Bot") : "Bot";
  },
  controls: {
    keyboard: "Click a piece, then where it goes",
    touch: "Tap a piece, then where it goes",
  },
  describeEnd(view) {
    if (view.game.case !== "checkers") return null;
    const end = view.game.value.end;
    return end ? endReason(end.reason) : null;
  },
};
