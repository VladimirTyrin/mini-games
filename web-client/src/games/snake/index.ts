import type { GamePlugin } from "../game-plugin";
import { SnakeBotKind, SnakeDeadSnakeBehavior, SnakeWallCollisionMode } from "@/proto/minigames/v2/snake_pb";

import Board from "./SnakeBoard.vue";
import SettingsForm from "./SnakeSettings.vue";

export const snakePlugin: GamePlugin = {
  id: "snake",
  settingsCase: "snake",
  title: "Snake",
  icon: "🐍",
  tagline: "Classic snake, up to 10 players",
  SettingsForm,
  Board,
  describeSettings(settings) {
    if (settings.game.case !== "snake") return [];
    const value = settings.game.value;
    return [
      `${value.fieldWidth}×${value.fieldHeight}`,
      value.wallCollisionMode === SnakeWallCollisionMode.WRAP_AROUND ? "wrap-around" : "walls kill",
      value.deadSnakeBehavior === SnakeDeadSnakeBehavior.STAY_ON_FIELD
        ? "dead snakes stay"
        : "dead snakes vanish",
      `${value.tickIntervalMs} ms tick`,
    ];
  },
  botLabel(kind) {
    if (kind.game.case !== "snake") return "Bot";
    return kind.game.value === SnakeBotKind.RANDOM ? "Random bot" : "Efficient bot";
  },
  controls: {
    keyboard: "Arrow keys or WASD to turn",
    touch: "Swipe on the field or use the arrows below it to turn",
  },
};
