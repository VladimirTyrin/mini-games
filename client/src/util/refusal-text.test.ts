import { create } from "@bufbuild/protobuf";
import { afterEach, describe, expect, it } from "vitest";

import { setLocale } from "@/i18n";
import {
  CommonRefusal,
  RefusalSchema,
  SettingProblemSchema,
  SettingRule,
} from "@/proto/minigames/v2/refusal_pb";
import type { CheckersRefusal } from "@/proto/minigames/v2/checkers_pb";
import { WordHuntRefusal } from "@/proto/minigames/v2/word_hunt_pb";

import { commonRefusal } from "@/match/common-refusal";

import { refusalText, settingProblemText } from "./refusal-text";

describe("refusal words", () => {
  afterEach(() => setLocale("en"));

  it("word a game's own reason in the player's language", () => {
    const tried = create(RefusalSchema, {
      reason: { case: "wordHunt", value: WordHuntRefusal.ALREADY_TRIED },
    });
    expect(refusalText(tried)).toBe("This word was already tried");
    setLocale("ru");
    expect(refusalText(tried)).toBe("Это слово уже было");
    expect(refusalText(commonRefusal(CommonRefusal.NOT_YOUR_TURN))).toBe("Сейчас не твой ход");
  });

  it("word a reason this build does not know yet in the core's English, else by its raw code", () => {
    const newer = create(RefusalSchema, { reason: { case: "checkers", value: 99 as CheckersRefusal } });
    setLocale("ru");
    expect(refusalText(newer, "the king may not jump twice")).toBe("the king may not jump twice");
    expect(refusalText(newer)).toBe("Так нельзя (checkers 99)");
    expect(refusalText(undefined)).toBe("Так нельзя");
  });

  it("word settings problems with their numbers", () => {
    const range = create(SettingProblemSchema, {
      field: "width",
      problem: { case: "outOfRange", value: { value: 2, min: 3, max: 20 } },
    });
    const rule = create(SettingProblemSchema, {
      field: "target",
      problem: { case: "brokenRule", value: { rule: SettingRule.POWER_OF_TWO, value: 100 } },
    });
    expect(settingProblemText(range)).toBe("Must be from 3 to 20");
    setLocale("ru");
    expect(settingProblemText(rule)).toBe("Должно быть степенью двойки");
  });
});
