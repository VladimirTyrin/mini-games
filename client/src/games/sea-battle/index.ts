import type { GamePlugin } from "../game-plugin";
import { SeaBattleBotKind, SeaBattlePhase } from "@/proto/minigames/v2/sea_battle_pb";

import Board from "./SeaBattleBoard.vue";
import SeaBattleIcon from "./SeaBattleIcon.vue";
import SettingsForm from "./SeaBattleSettings.vue";

const BOT_LABELS: Record<number, string> = {
  [SeaBattleBotKind.EASY]: "Easy bot",
  [SeaBattleBotKind.MEDIUM]: "Medium bot",
  [SeaBattleBotKind.HARD]: "Hard bot",
};

export const seaBattlePlugin: GamePlugin = {
  id: "sea_battle",
  settingsCase: "seaBattle",
  title: "Sea Battle",
  icon: "🚢",
  Icon: SeaBattleIcon,
  tagline: "Find and sink the hidden fleet",
  SettingsForm,
  Board,
  describeSettings: () => ["classic rules"],
  botLabel(kind) {
    return kind.game.case === "seaBattle" ? (BOT_LABELS[kind.game.value] ?? "Bot") : "Bot";
  },
  controls: {
    keyboard: "Place your fleet, then click a cell of the other sea to fire",
    touch: "Place your fleet, then tap a cell of the other sea to fire",
  },
  describeEnd(view) {
    if (view.game.case !== "seaBattle") return null;
    switch (view.game.value.phase) {
      case SeaBattlePhase.WON:
        return "The whole fleet is sunk";
      case SeaBattlePhase.FORFEITED:
        return "A player gave up or left";
      default:
        return null;
    }
  },
};
