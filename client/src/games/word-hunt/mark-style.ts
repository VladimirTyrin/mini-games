import { WordHuntLetterMark } from "@/proto/minigames/v2/word_hunt_pb";

import { usesHighContrast } from "./preferences";

/** The in-place colour, also for the word shown at the end. */
export function inPlaceClass(): string {
  return usesHighContrast.value ? "bg-orange-500 text-white" : "bg-emerald-600 text-white";
}

/**
 * Tile and key colours: grey absent, amber elsewhere in the word, green in place (blue and
 * orange with high contrast). Reads the preference, so templates follow a change at once.
 */
export function markClass(mark: WordHuntLetterMark): string {
  switch (mark) {
    case WordHuntLetterMark.CORRECT:
      return inPlaceClass();
    case WordHuntLetterMark.PRESENT:
      return usesHighContrast.value ? "bg-sky-500 text-white" : "bg-amber-500 text-white";
    case WordHuntLetterMark.ABSENT:
      return "bg-gray-700 text-gray-300";
    default:
      return "bg-gray-500/40 text-gray-100";
  }
}
