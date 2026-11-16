import { storage } from "@/platform/storage";
import type { WordHuntLanguage } from "@/proto/minigames/v2/word_hunt_pb";

const KEY = "mini-games.word-hunt.stats.v2";
/** How many counted games are remembered so that showing one again does not count it again. */
const REMEMBERED_GAMES = 20;

/** One language's results on this device. */
export interface WordHuntStats {
  played: number;
  won: number;
  /** Wins in a row, current and best. */
  streak: number;
  bestStreak: number;
  /** `wins[n - 1]`: games solved in `n` guesses. */
  wins: number[];
  /** The last word of the day taken on: it counts once, however often it is replayed. */
  lastDailyDay: number | null;
  /** A game with a guess and no result yet; leaving it counts as a loss when the next starts. */
  unfinished: string | null;
  /** Games already counted (newest last). */
  counted: string[];
}

/** One match of the game, as the stats know it. */
export interface StatsGame {
  language: WordHuntLanguage;
  /** Unique per match (see `matchKey`). */
  key: string;
  /** Set for the word of the day. */
  dailyDay: number | null;
}

type StatsByLanguage = Partial<Record<WordHuntLanguage, WordHuntStats>>;

function empty(): WordHuntStats {
  return {
    played: 0,
    won: 0,
    streak: 0,
    bestStreak: 0,
    wins: [],
    lastDailyDay: null,
    unfinished: null,
    counted: [],
  };
}

function load(): StatsByLanguage {
  try {
    const raw = storage.get(KEY);
    return raw ? (JSON.parse(raw) as StatsByLanguage) : {};
  } catch {
    return {};
  }
}

function save(all: StatsByLanguage): void {
  storage.set(KEY, JSON.stringify(all));
}

function count(stats: WordHuntStats, key: string, solvedIn: number | null): void {
  stats.played += 1;
  if (solvedIn === null) {
    stats.streak = 0;
  } else {
    stats.won += 1;
    stats.streak += 1;
    stats.bestStreak = Math.max(stats.bestStreak, stats.streak);
    const index = solvedIn - 1;
    while (stats.wins.length <= index) stats.wins.push(0);
    stats.wins[index] = (stats.wins[index] ?? 0) + 1;
  }
  stats.counted = [...stats.counted, key].slice(-REMEMBERED_GAMES);
  if (stats.unfinished === key) stats.unfinished = null;
}

export function statsFor(language: WordHuntLanguage): WordHuntStats {
  return load()[language] ?? empty();
}

/**
 * The player made a guess in `game`: from now on leaving it is a loss. A word of the day that
 * was already taken on is not counted again (so leaving and restarting it gains nothing).
 */
export function startGame(game: StatsGame): void {
  const all = load();
  const stats = all[game.language] ?? empty();
  if (stats.unfinished === game.key || stats.counted.includes(game.key)) return;
  if (stats.unfinished !== null) count(stats, stats.unfinished, null);
  if (game.dailyDay !== null) {
    if (stats.lastDailyDay !== null && game.dailyDay <= stats.lastDailyDay) {
      all[game.language] = stats;
      save(all);
      return;
    }
    stats.lastDailyDay = game.dailyDay;
  }
  stats.unfinished = game.key;
  all[game.language] = stats;
  save(all);
}

/** Where a game stands: still going, solved in so many guesses, or lost (given up included). */
export type StatsProgress =
  | { readonly kind: "playing" }
  | { readonly kind: "solved"; readonly guesses: number }
  | { readonly kind: "lost" };

/**
 * Records `game` as it stands after `guessCount` guesses; returns the stats once it is over. A
 * game counts from its first guess; a word of the day given up before one counts too, since the
 * word is then shown and replaying the day would be a sure win.
 */
export function recordProgress(
  game: StatsGame,
  guessCount: number,
  progress: StatsProgress,
): WordHuntStats | null {
  const revealedTheDailyWord = progress.kind === "lost" && game.dailyDay !== null;
  if (guessCount === 0 && !revealedTheDailyWord) return null;
  startGame(game);
  if (progress.kind === "solved") return finishGame(game, progress.guesses);
  if (progress.kind === "lost") return finishGame(game, null);
  return null;
}

/** `game` ended; counted only if it was started (see `startGame`) and not counted yet. */
export function finishGame(game: StatsGame, solvedIn: number | null): WordHuntStats {
  const all = load();
  const stats = all[game.language] ?? empty();
  if (stats.unfinished !== game.key) return stats;
  count(stats, game.key, solvedIn);
  all[game.language] = stats;
  save(all);
  return stats;
}
