import type { Preset } from "@/components/form/presets";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";

interface BoardPatch {
  fieldWidth: number;
  fieldHeight: number;
  winCount: number;
}

export const boardPresets: readonly Preset<BoardPatch>[] = [
  {
    key: "classic",
    label: "3×3",
    detail: "3 in a row",
    patch: { fieldWidth: 3, fieldHeight: 3, winCount: 3 },
  },
  {
    key: "medium",
    label: "9×9",
    detail: "5 in a row",
    patch: { fieldWidth: 9, fieldHeight: 9, winCount: 5 },
  },
  {
    key: "large",
    label: "15×15",
    detail: "5 in a row",
    patch: { fieldWidth: 15, fieldHeight: 15, winCount: 5 },
  },
];

const PHONE_BOARD = "medium";

/** A 15×15 board leaves ~21 px cells on a phone; start phones on 9×9 instead. */
export function narrowScreenSettings(defaults: GameSettings): GameSettings {
  const preset = boardPresets.find((candidate) => candidate.key === PHONE_BOARD);
  if (defaults.game.case !== "tictactoe" || !preset) return defaults;
  return { ...defaults, game: { case: "tictactoe", value: { ...defaults.game.value, ...preset.patch } } };
}
