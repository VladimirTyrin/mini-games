import { storage } from "@/platform/storage";
import type { WordHuntLanguage } from "@/proto/minigames/v2/word_hunt_pb";

const KEY = "mini-games.word-hunt.stats.v1";

/** One language's results on this device. */
export interface WordHuntStats {
  played: number;
  won: number;
  /** Wins in a row, current and best. */
  streak: number;
  bestStreak: number;
  /** `wins[n - 1]`: games solved in `n` guesses. */
  wins: number[];
  /** The last word of the day counted, so replaying it does not count twice. */
  lastDailyDay: number | null;
}

export interface FinishedGame {
  language: WordHuntLanguage;
  /** Guesses used; `null` when the word was not found. */
  solvedIn: number | null;
  /** Set for the word of the day. */
  dailyDay: number | null;
}

type StatsByLanguage = Partial<Record<WordHuntLanguage, WordHuntStats>>;

function empty(): WordHuntStats {
  return { played: 0, won: 0, streak: 0, bestStreak: 0, wins: [], lastDailyDay: null };
}

function load(): StatsByLanguage {
  try {
    const raw = storage.get(KEY);
    return raw ? (JSON.parse(raw) as StatsByLanguage) : {};
  } catch {
    return {};
  }
}

export function statsFor(language: WordHuntLanguage): WordHuntStats {
  return load()[language] ?? empty();
}

/** Counts a finished game; a word of the day counts once. Returns the updated stats. */
export function recordGame(game: FinishedGame): WordHuntStats {
  const all = load();
  const stats = all[game.language] ?? empty();
  if (game.dailyDay !== null) {
    if (stats.lastDailyDay !== null && game.dailyDay <= stats.lastDailyDay) return stats;
    stats.lastDailyDay = game.dailyDay;
  }
  stats.played += 1;
  if (game.solvedIn === null) {
    stats.streak = 0;
  } else {
    stats.won += 1;
    stats.streak += 1;
    stats.bestStreak = Math.max(stats.bestStreak, stats.streak);
    const index = game.solvedIn - 1;
    while (stats.wins.length <= index) stats.wins.push(0);
    stats.wins[index] = (stats.wins[index] ?? 0) + 1;
  }
  all[game.language] = stats;
  storage.set(KEY, JSON.stringify(all));
  return stats;
}
