import { t } from "@/i18n";
import type { Refusal, SettingProblem } from "@/proto/minigames/v2/refusal_pb";
import type { SettingsProblem } from "@/wasm/core";

/**
 * Why the rules refuse a move, in the player's language. A reason this build does not know (a
 * newer server's) falls back to `english`, the core's own words that came with it, else to a
 * generic line.
 */
export function refusalText(refusal: Refusal | undefined, english?: string): string {
  const texts = t.refusals;
  const reason = refusal?.reason;
  const worded: string | undefined = (() => {
    switch (reason?.case) {
      case "common":
        return texts.common[reason.value];
      case "snake":
        return texts.snake[reason.value];
      case "tictactoe":
        return texts.tictactoe[reason.value];
      case "puzzle2048":
        return texts.puzzle2048[reason.value];
      case "numbersMatch":
        return texts.numbersMatch[reason.value];
      case "wordHunt":
        return texts.wordHunt[reason.value];
      case "checkers":
        return texts.checkers[reason.value];
      case "seaBattle":
        return texts.seaBattle[reason.value];
      // xtask:refusals
      default:
        return undefined;
    }
  })();
  if (worded !== undefined) return worded;
  // A value of an enum this build does not know yet indexes nothing: the core's English, else
  // the raw code, so the player can still report it.
  if (english) return english;
  return reason?.case ? `${texts.notAllowed} (${reason.case} ${reason.value})` : texts.notAllowed;
}

/** What is wrong with one setting, in the player's language; `english` as in `refusalText`. */
export function settingProblemText(problem: SettingProblem, english?: string): string {
  const texts = t.refusals;
  switch (problem.problem.case) {
    case "outOfRange":
      return texts.outOfRange(problem.problem.value.min, problem.problem.value.max);
    case "brokenRule":
      return (
        texts.settingRules[problem.problem.value.rule] ??
        (english || `${texts.settingRules[0]} (${problem.problem.value.rule})`)
      );
    default:
      return english || texts.settingRules[0];
  }
}

/** Why settings cannot be used, in the player's language. */
export function settingsProblemText(problem: SettingsProblem): string {
  return problem.detail ? settingProblemText(problem.detail) : t.refusals.doNotSuitPlayers;
}
