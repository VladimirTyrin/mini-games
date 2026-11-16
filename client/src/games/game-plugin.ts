import type { Component } from "vue";

import type { BotKind, GameSettings, GameView } from "@/proto/minigames/v2/game_pb";
import { prefersTouch } from "@/input/prefers-touch";

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
  /** Emoji shown where the game has no `Icon` (e.g. a freshly scaffolded game). */
  icon: string;
  /** The game's picture in lists and headers (square SVG). */
  Icon?: Component;
  tagline: string;
  SettingsForm: Component;
  Board: Component;
  /**
   * Short human-readable facts for lobby cards, e.g. "15×15", "5 in a row". `seats`: taken seats
   * when known (a lobby), so facts that only apply alone or in a group fit.
   */
  describeSettings(settings: GameSettings, seats?: number): string[];
  botLabel(kind: BotKind): string;
  /** How to play, shown under the board and on the setup screen; see `controlsHintFor`. */
  controls: ControlsHint;
  /** Space is a game key (Stack Attack's jump): it must not start the next match at the end. */
  playsWithSpace?: true;
  /** Starting settings on a phone when none are remembered; the core defaults otherwise. */
  narrowScreenSettings?(defaults: GameSettings): GameSettings;
  /**
   * The settings a match starting now uses, from the ones picked on the setup screen (Word Hunt
   * gives the word of the day today's date). Applied to every local start, Play again included.
   */
  settingsForNewMatch?(settings: GameSettings): GameSettings;
  /** Why the match in `view` ended ("Crushed by a falling crate"), shown with the result. */
  describeEnd?(view: GameView): string | null;
}

export interface ControlsHint {
  readonly keyboard: string;
  readonly touch: string;
}

/** The hint for the device at hand. */
export function controlsHintFor(plugin: GamePlugin): string {
  return prefersTouch() ? plugin.controls.touch : plugin.controls.keyboard;
}
