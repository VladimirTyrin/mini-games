import { beforeEach, describe, expect, it } from "vitest";

import { storage } from "@/platform/storage";
import { create } from "@bufbuild/protobuf";

import { GameSettingsSchema } from "@/proto/minigames/v2/game_pb";
import { WordHuntLanguage, WordHuntLetterMark, WordHuntPuzzleMode } from "@/proto/minigames/v2/word_hunt_pb";

import { formatDay, localDayNumber } from "./daily";
import { keyboardRows, letterForKey } from "./keyboard-layout";
import { markClass } from "./mark-style";
import { usesHighContrast } from "./preferences";
import { finishGame, recordProgress, startGame, statsFor } from "./stats";
import { wordHuntPlugin } from "./index";
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
    storage.remove("mini-games.word-hunt.stats.v2");
  });

  const russian = WordHuntLanguage.RUSSIAN;
  const game = (key: string, dailyDay: number | null = null) => ({ language: russian, key, dailyDay });
  const play = (key: string, solvedIn: number | null, dailyDay: number | null = null) => {
    startGame(game(key, dailyDay));
    return finishGame(game(key, dailyDay), solvedIn);
  };

  it("counts wins, losses and streaks per language", () => {
    play("a", 3);
    play("b", 4);
    play("c", null);
    play("d", 3);
    expect(statsFor(russian)).toMatchObject({
      played: 4,
      won: 3,
      streak: 1,
      bestStreak: 2,
      wins: [0, 0, 2, 1],
    });
    expect(statsFor(WordHuntLanguage.ENGLISH).played).toBe(0);
  });

  it("counts a game once, however often its result is shown", () => {
    play("a", 2);
    startGame(game("a"));
    finishGame(game("a"), 2);
    expect(statsFor(russian).played).toBe(1);
  });

  it("counts a game left after a guess as lost once the next one starts", () => {
    play("a", 2);
    startGame(game("left"));
    expect(statsFor(russian).played).toBe(1);
    startGame(game("next"));
    expect(statsFor(russian)).toMatchObject({ played: 2, won: 1, streak: 0 });
  });

  it("counts a word of the day given up before any guess, so replaying the day gains nothing", () => {
    recordProgress(game("gave up", 20_000), 0, { kind: "lost" });
    play("replayed", 1, 20_000);
    expect(statsFor(russian)).toMatchObject({ played: 1, won: 0, streak: 0 });
  });

  it("does not count an unlimited word given up before any guess", () => {
    recordProgress(game("gave up"), 0, { kind: "lost" });
    expect(statsFor(russian).played).toBe(0);
  });

  it("counts the word of the day once, even when it is left and restarted", () => {
    startGame(game("first try", 20_000));
    play("second try", 1, 20_000);
    play("tomorrow", 2, 20_001);
    expect(statsFor(russian)).toMatchObject({ played: 2, won: 1, lastDailyDay: 20_001 });
  });
});

describe("plugin", () => {
  it("gives the word of the day today's date at every start", () => {
    const remembered = create(GameSettingsSchema, {
      game: { case: "wordHunt", value: { mode: WordHuntPuzzleMode.DAILY, day: 5, wordLength: 5 } },
    });
    const started = wordHuntPlugin.settingsForNewMatch?.(remembered);
    expect(started?.game.case === "wordHunt" && started.game.value.day).toBe(localDayNumber());
  });
});

describe("mark colours", () => {
  it("tell in place from elsewhere by more than green against amber with high contrast", () => {
    usesHighContrast.value = true;
    try {
      expect(markClass(WordHuntLetterMark.CORRECT)).toContain("orange");
      expect(markClass(WordHuntLetterMark.PRESENT)).toContain("sky");
    } finally {
      usesHighContrast.value = false;
    }
    expect(markClass(WordHuntLetterMark.CORRECT)).toContain("emerald");
  });
});
