import { WordHuntLanguage, WordHuntPhase, WordHuntPuzzleMode } from "@/proto/minigames/v2/word_hunt_pb";

import type { GamePlugin } from "../game-plugin";

import { localDayNumber } from "./daily";
import Board from "./WordHuntBoard.vue";
import WordHuntIcon from "./WordHuntIcon.vue";
import SettingsForm from "./WordHuntSettings.vue";

export const wordHuntPlugin: GamePlugin = {
  id: "word_hunt",
  settingsCase: "wordHunt",
  title: "Word Hunt",
  icon: "🔤",
  Icon: WordHuntIcon,
  tagline: "Guess the word: Russian or English",
  SettingsForm,
  Board,
  describeSettings(settings) {
    if (settings.game.case !== "wordHunt") return [];
    const value = settings.game.value;
    const facts = [
      value.language === WordHuntLanguage.RUSSIAN ? "Русский" : "English",
      value.mode === WordHuntPuzzleMode.DAILY ? "word of the day" : `${value.wordLength} letters`,
      `${value.maxGuesses} guesses`,
    ];
    return value.hardMode ? [...facts, "hard mode"] : facts;
  },
  botLabel: () => "Bot",
  controls: {
    keyboard: "Type a word: it is checked as soon as its last letter is in",
    touch: "Tap the letters: the word is checked as soon as its last letter is in",
  },
  settingsForNewMatch(settings) {
    // The word of the day is today's, even with settings remembered from an earlier day.
    if (settings.game.case !== "wordHunt" || settings.game.value.mode !== WordHuntPuzzleMode.DAILY) {
      return settings;
    }
    return {
      ...settings,
      game: { case: "wordHunt", value: { ...settings.game.value, day: localDayNumber() } },
    };
  },
  describeEnd(view) {
    if (view.game.case !== "wordHunt") return null;
    const { answer, phase, guesses } = view.game.value;
    if (phase === WordHuntPhase.SOLVED) return `Found in ${guesses.length}`;
    return answer ? `The word was ${answer.toUpperCase()}` : null;
  },
};
