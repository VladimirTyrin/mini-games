import {
  WordHuntBotKind,
  WordHuntFirstTurn,
  WordHuntLanguage,
  WordHuntPhase,
  WordHuntPuzzleMode,
} from "@/proto/minigames/v2/word_hunt_pb";

import { locale, t } from "@/i18n";

import type { GamePlugin } from "../game-plugin";

import { localDayNumber } from "./daily";
import Board from "./WordHuntBoard.vue";
import HelpEn from "./WordHuntHelp.en.vue";
import HelpRu from "./WordHuntHelp.ru.vue";
import WordHuntIcon from "./WordHuntIcon.vue";
import SettingsForm from "./WordHuntSettings.vue";

export const wordHuntPlugin: GamePlugin = {
  id: "word_hunt",
  settingsCase: "wordHunt",
  get title() {
    return t.games.wordHunt.title;
  },
  icon: "🔤",
  Icon: WordHuntIcon,
  get tagline() {
    return t.games.wordHunt.tagline;
  },
  SettingsForm,
  Board,
  Help: { en: HelpEn, ru: HelpRu },
  describeSettings(settings, seats) {
    if (settings.game.case !== "wordHunt") return [];
    const value = settings.game.value;
    const texts = t.games.wordHunt;
    const facts = [
      value.language === WordHuntLanguage.RUSSIAN ? texts.wordLanguages.russian : texts.wordLanguages.english,
      value.mode === WordHuntPuzzleMode.DAILY
        ? texts.facts.wordOfTheDay
        : texts.facts.letters(value.wordLength),
    ];
    // Alone the guesses are limited; a group plays until someone finds the word.
    if (seats === undefined || seats <= 1) facts.push(texts.facts.guesses(value.maxGuesses));
    if (seats === undefined || seats > 1) {
      if (value.firstTurn === WordHuntFirstTurn.HOST_FIRST) facts.push(texts.facts.hostFirst);
      if (value.firstTurn === WordHuntFirstTurn.HOST_LAST) facts.push(texts.facts.hostLast);
    }
    if (value.hardMode) facts.push(texts.facts.hardMode);
    return facts;
  },
  botLabel(kind) {
    if (kind.game.case !== "wordHunt") return t.common.bot;
    switch (kind.game.value) {
      case WordHuntBotKind.EASY:
        return t.games.wordHunt.bots.easy;
      case WordHuntBotKind.HARD:
        return t.games.wordHunt.bots.hard;
      default:
        return t.games.wordHunt.bots.medium;
    }
  },
  controls: {
    get keyboard() {
      return t.games.wordHunt.controls.keyboard;
    },
    get touch() {
      return t.games.wordHunt.controls.touch;
    },
  },
  defaultSettings(defaults) {
    // A player who never picked a word list gets the interface's language; a remembered pick
    // (and a lobby's settings) never reach here.
    if (defaults.game.case !== "wordHunt") return defaults;
    const language = locale.value === "ru" ? WordHuntLanguage.RUSSIAN : WordHuntLanguage.ENGLISH;
    return { ...defaults, game: { case: "wordHunt", value: { ...defaults.game.value, language } } };
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
    if (phase === WordHuntPhase.SOLVED) return t.games.wordHunt.end.foundIn(guesses.length);
    return answer ? t.games.wordHunt.end.wordWas(answer.toUpperCase()) : null;
  },
};
