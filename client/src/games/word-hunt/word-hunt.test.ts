import { beforeEach, describe, expect, it } from "vitest";

import { storage } from "@/platform/storage";
import { WordHuntLanguage } from "@/proto/minigames/v2/word_hunt_pb";

import { formatDay, localDayNumber } from "./daily";
import { keyboardRows, letterForKey } from "./keyboard-layout";
import { recordGame, statsFor } from "./stats";
import { normalizeWord, parseWordList } from "./word-lists";

describe("word lists", () => {
  it("reads sections by length and skips comments", () => {
    const words = parseWordList("# source\nwords 4\nкота\nрыба\nwords 5\nкошка\n");
    expect([...(words.get(4) ?? [])]).toEqual(["кота", "рыба"]);
    expect(words.get(5)?.has("кошка")).toBe(true);
    expect(words.get(6)).toBeUndefined();
  });

  it("spells guesses the way the lists do", () => {
    expect(normalizeWord("ЁЛКА")).toBe("елка");
  });
});

describe("keyboard", () => {
  it("types Russian letters from either system layout", () => {
    expect(letterForKey(WordHuntLanguage.RUSSIAN, "ф", "KeyA")).toBe("ф");
    expect(letterForKey(WordHuntLanguage.RUSSIAN, "a", "KeyA")).toBe("ф");
    expect(letterForKey(WordHuntLanguage.RUSSIAN, "Ё", "Backquote")).toBe("е");
    expect(letterForKey(WordHuntLanguage.RUSSIAN, "1", "Digit1")).toBeNull();
  });

  it("types English letters from either system layout", () => {
    expect(letterForKey(WordHuntLanguage.ENGLISH, "Q", "KeyQ")).toBe("q");
    expect(letterForKey(WordHuntLanguage.ENGLISH, "й", "KeyQ")).toBe("q");
    expect(letterForKey(WordHuntLanguage.ENGLISH, "Enter", "Enter")).toBeNull();
  });

  it("lays out every letter of the alphabet except ё", () => {
    expect(keyboardRows(WordHuntLanguage.RUSSIAN).join("").length).toBe(32);
    expect(keyboardRows(WordHuntLanguage.ENGLISH).join("").length).toBe(26);
  });
});

describe("daily", () => {
  it("counts days on the local calendar", () => {
    expect(localDayNumber(new Date(1970, 0, 1, 23, 59))).toBe(0);
    expect(localDayNumber(new Date(2026, 10, 16, 0, 1))).toBe(localDayNumber(new Date(2026, 10, 16, 23, 59)));
    expect(localDayNumber(new Date(2026, 10, 17, 0, 0)) - localDayNumber(new Date(2026, 10, 16, 12, 0))).toBe(
      1,
    );
    expect(formatDay(localDayNumber(new Date(2026, 10, 16)))).toContain("16");
  });
});

describe("stats", () => {
  beforeEach(() => {
    storage.remove("mini-games.word-hunt.stats.v1");
  });

  it("counts wins, losses and streaks per language", () => {
    const russian = WordHuntLanguage.RUSSIAN;
    recordGame({ language: russian, solvedIn: 3, dailyDay: null });
    recordGame({ language: russian, solvedIn: 4, dailyDay: null });
    recordGame({ language: russian, solvedIn: null, dailyDay: null });
    recordGame({ language: russian, solvedIn: 3, dailyDay: null });
    expect(statsFor(russian)).toMatchObject({
      played: 4,
      won: 3,
      streak: 1,
      bestStreak: 2,
      wins: [0, 0, 2, 1],
    });
    expect(statsFor(WordHuntLanguage.ENGLISH).played).toBe(0);
  });

  it("counts a word of the day once", () => {
    const english = WordHuntLanguage.ENGLISH;
    recordGame({ language: english, solvedIn: 2, dailyDay: 20_000 });
    recordGame({ language: english, solvedIn: null, dailyDay: 20_000 });
    expect(statsFor(english)).toMatchObject({ played: 1, won: 1, lastDailyDay: 20_000 });
  });
});
