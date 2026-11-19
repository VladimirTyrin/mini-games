import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { defaultSettings } from "@/wasm/core";

import type { GamePlugin } from "./game-plugin";

// Tailwind's `sm` breakpoint: below it the setup screen is a phone layout.
const NARROW_SCREEN = "(max-width: 639px)";

/** What a setup form starts from: the last settings used, else defaults that suit the screen. */
export function initialSettings(plugin: GamePlugin, remembered: GameSettings | null): GameSettings {
  // Settings of another game saved under this one's id (an old bug) are ignored, not loaded.
  if (remembered && remembered.game.case === plugin.settingsCase) return remembered;
  const defaults = plugin.defaultSettings?.(defaultSettings(plugin.id)) ?? defaultSettings(plugin.id);
  const isNarrow = typeof window !== "undefined" && window.matchMedia(NARROW_SCREEN).matches;
  return isNarrow && plugin.narrowScreenSettings ? plugin.narrowScreenSettings(defaults) : defaults;
}
