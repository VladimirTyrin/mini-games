import type { GamePlugin } from "../game-plugin";
import { t } from "@/i18n";
import { SnakeBotKind, SnakeDeadSnakeBehavior, SnakeWallCollisionMode } from "@/proto/minigames/v2/snake_pb";

import Board from "./SnakeBoard.vue";
import HelpEn from "./SnakeHelp.en.vue";
import HelpRu from "./SnakeHelp.ru.vue";
import SettingsForm from "./SnakeSettings.vue";
import SnakeIcon from "./SnakeIcon.vue";
import { snakeSpeedName } from "./speeds";

export const snakePlugin: GamePlugin = {
  id: "snake",
  settingsCase: "snake",
  get title() {
    return t.games.snake.title;
  },
  icon: "🐍",
  Icon: SnakeIcon,
  get tagline() {
    return t.games.snake.tagline;
  },
  SettingsForm,
  Board,
  Help: { en: HelpEn, ru: HelpRu },
  describeSettings(settings) {
    if (settings.game.case !== "snake") return [];
    const value = settings.game.value;
    const facts = t.games.snake.facts;
    return [
      `${value.fieldWidth}×${value.fieldHeight}`,
      value.wallCollisionMode === SnakeWallCollisionMode.WRAP_AROUND ? facts.wrapAround : facts.wallsKill,
      value.deadSnakeBehavior === SnakeDeadSnakeBehavior.STAY_ON_FIELD
        ? facts.deadSnakesStay
        : facts.deadSnakesVanish,
      snakeSpeedName(value.tickIntervalMs).toLowerCase(),
    ];
  },
  botLabel(kind) {
    if (kind.game.case !== "snake") return t.common.bot;
    switch (kind.game.value) {
      case SnakeBotKind.RANDOM:
        return t.games.snake.bots.random;
      case SnakeBotKind.CAREFUL:
        return t.games.snake.bots.careful;
      default:
        return t.games.snake.bots.efficient;
    }
  },
  controls: {
    get keyboard() {
      return t.games.snake.controls.keyboard;
    },
    get touch() {
      return t.games.snake.controls.touch;
    },
  },
};
