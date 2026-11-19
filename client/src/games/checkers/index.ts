import { t } from "@/i18n";
import type { GamePlugin } from "../game-plugin";
import {
  CheckersBotKind,
  CheckersCaptureRule,
  CheckersEndReason,
  CheckersWhitePlayer,
} from "@/proto/minigames/v2/checkers_pb";

import Board from "./CheckersBoard.vue";
import HelpEn from "./CheckersHelp.en.vue";
import HelpRu from "./CheckersHelp.ru.vue";
import CheckersIcon from "./CheckersIcon.vue";
import SettingsForm from "./CheckersSettings.vue";

function whiteLabel(white: CheckersWhitePlayer): string {
  switch (white) {
    case CheckersWhitePlayer.HOST:
      return t.games.checkers.facts.hostPlaysWhite;
    case CheckersWhitePlayer.GUEST:
      return t.games.checkers.facts.guestPlaysWhite;
    default:
      return t.games.checkers.facts.coloursByLot;
  }
}

function botLabel(kind: CheckersBotKind): string {
  switch (kind) {
    case CheckersBotKind.EASY:
      return t.games.checkers.bots.easy;
    case CheckersBotKind.MEDIUM:
      return t.games.checkers.bots.medium;
    case CheckersBotKind.HARD:
      return t.games.checkers.bots.hard;
    case CheckersBotKind.MASTER:
      return t.games.checkers.bots.master;
    default:
      return t.common.bot;
  }
}

/** Why the game ended, shown with the result. */
export function endReason(reason: CheckersEndReason): string | null {
  const texts = t.games.checkers.endReasons;
  switch (reason) {
    case CheckersEndReason.NO_PIECES:
      return texts.noPieces;
    case CheckersEndReason.NO_MOVES:
      return texts.noMoves;
    case CheckersEndReason.OPPONENT_LEFT:
      return texts.opponentLeft;
    case CheckersEndReason.REPETITION:
      return texts.repetition;
    case CheckersEndReason.KING_MOVES:
      return texts.kingMoves;
    default:
      return null;
  }
}

export const checkersPlugin: GamePlugin = {
  id: "checkers",
  settingsCase: "checkers",
  get title() {
    return t.games.checkers.title;
  },
  icon: "⚪",
  Icon: CheckersIcon,
  get tagline() {
    return t.games.checkers.tagline;
  },
  SettingsForm,
  Board,
  Help: { en: HelpEn, ru: HelpRu },
  describeSettings(settings) {
    if (settings.game.case !== "checkers") return [];
    const { whitePlayer, captureRule } = settings.game.value;
    return captureRule === CheckersCaptureRule.MAY_STOP
      ? [whiteLabel(whitePlayer), t.games.checkers.facts.capturesMayStop]
      : [whiteLabel(whitePlayer)];
  },
  botLabel(kind) {
    return kind.game.case === "checkers" ? botLabel(kind.game.value) : t.common.bot;
  },
  controls: {
    get keyboard() {
      return t.games.checkers.controls.keyboard;
    },
    get touch() {
      return t.games.checkers.controls.touch;
    },
  },
  describeEnd(view) {
    if (view.game.case !== "checkers") return null;
    const end = view.game.value.end;
    return end ? endReason(end.reason) : null;
  },
};
