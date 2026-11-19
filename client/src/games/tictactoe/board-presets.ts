import type { Preset } from "@/components/form/presets";
import { t } from "@/i18n";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";

interface BoardPatch {
  fieldWidth: number;
  fieldHeight: number;
  winCount: number;
}

const BOARD_PATCHES: readonly { key: string; patch: BoardPatch }[] = [
  { key: "classic", patch: { fieldWidth: 3, fieldHeight: 3, winCount: 3 } },
  { key: "medium", patch: { fieldWidth: 9, fieldHeight: 9, winCount: 5 } },
  { key: "large", patch: { fieldWidth: 15, fieldHeight: 15, winCount: 5 } },
];

/** The board chips, worded in the current language. */
export function boardPresets(): Preset<BoardPatch>[] {
  return BOARD_PATCHES.map(({ key, patch }) => ({
    key,
    label: `${patch.fieldWidth}×${patch.fieldHeight}`,
    detail: t.games.tictactoe.inARow(patch.winCount),
    patch,
  }));
}

const PHONE_BOARD = "medium";

/** A 15×15 board leaves ~21 px cells on a phone; start phones on 9×9 instead. */
export function narrowScreenSettings(defaults: GameSettings): GameSettings {
  const preset = BOARD_PATCHES.find((candidate) => candidate.key === PHONE_BOARD);
  if (defaults.game.case !== "tictactoe" || !preset) return defaults;
  return { ...defaults, game: { case: "tictactoe", value: { ...defaults.game.value, ...preset.patch } } };
}
