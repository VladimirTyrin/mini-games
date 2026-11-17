import { WordHuntLetterMark } from "@/proto/minigames/v2/word_hunt_pb";

/** The in-place colour, also for the word shown at the end. */
export function inPlaceClass(): string {
  return "bg-emerald-600 text-white";
}

/** Tile and key colours: grey absent, amber elsewhere in the word, green in place. */
export function markClass(mark: WordHuntLetterMark): string {
  switch (mark) {
    case WordHuntLetterMark.CORRECT:
      return inPlaceClass();
    case WordHuntLetterMark.PRESENT:
      return "bg-amber-500 text-white";
    case WordHuntLetterMark.ABSENT:
      return "bg-gray-700 text-gray-300";
    default:
      return "bg-gray-500/40 text-gray-100";
  }
}
