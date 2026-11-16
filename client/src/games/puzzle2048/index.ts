import type { GamePlugin } from "../game-plugin";

import Board from "./Puzzle2048Board.vue";
import SettingsForm from "./Puzzle2048Settings.vue";
import Puzzle2048Icon from "./Puzzle2048Icon.vue";

export const puzzle2048Plugin: GamePlugin = {
  id: "puzzle2048",
  settingsCase: "puzzle2048",
  title: "2048",
  icon: "🔢",
  Icon: Puzzle2048Icon,
  tagline: "Slide and merge tiles to reach the target",
  SettingsForm,
  Board,
  describeSettings(settings) {
    if (settings.game.case !== "puzzle2048") return [];
    const value = settings.game.value;
    return [`${value.width}×${value.height}`, `to ${value.target}`];
  },
  botLabel: () => "Bot",
  controls: {
    keyboard: "Arrow keys or WASD to slide the tiles",
    touch: "Swipe to slide the tiles",
  },
};
