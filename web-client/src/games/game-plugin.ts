import type { Component } from "vue";

import type { BotKind, GameSettings, GameView } from "@/proto/minigames/v2/game_pb";

export type SettingsCase = NonNullable<GameSettings["game"]["case"]>;

/**
 * Everything the UI needs to know about one game. Rules, limits and validation come from the
 * WASM core; a plugin only renders and maps input to commands.
 *
 * - `SettingsForm` — props `{ modelValue: GameSettings }`, emits `update:modelValue`; rendered
 *   inside `SettingsEditor`, builds its rows from `components/form` with
 *   `useGameSettingsForm` (limits and errors from the core).
 * - `Board` — props `{ controller: MatchController }`; renders `controller.current` (and
 *   interpolates from `controller.previous` for real-time games) and calls
 *   `controller.submit(...)` for input.
 */
export interface GamePlugin {
  /** Core game id (`GameInfo.id`). */
  id: string;
  settingsCase: SettingsCase;
  title: string;
  icon: string;
  tagline: string;
  SettingsForm: Component;
  Board: Component;
  /** Short human-readable facts for lobby cards, e.g. "15×15", "5 in a row". */
  describeSettings(settings: GameSettings): string[];
  botLabel(kind: BotKind): string;
  /** Keyboard help shown under the board and on the setup screen. */
  controlsHint: string;
  /** Starting settings on a phone when none are remembered; the core defaults otherwise. */
  narrowScreenSettings?(defaults: GameSettings): GameSettings;
  /** Why the match in `view` ended ("Crushed by a falling crate"), shown with the result. */
  describeEnd?(view: GameView): string | null;
}
