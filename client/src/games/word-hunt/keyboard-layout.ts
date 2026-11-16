import { WordHuntLanguage } from "@/proto/minigames/v2/word_hunt_pb";

/** On-screen letter rows (ЙЦУКЕН and QWERTY); `ё` is typed as `е`, as the game spells it. */
export function keyboardRows(language: WordHuntLanguage): readonly string[] {
  return language === WordHuntLanguage.RUSSIAN
    ? ["йцукенгшщзхъ", "фывапролджэ", "ячсмитьбю"]
    : ["qwertyuiop", "asdfghjkl", "zxcvbnm"];
}

/** Russian letters by physical key (`KeyboardEvent.code`) on the standard ЙЦУКЕН layout. */
const RUSSIAN_BY_CODE: Readonly<Record<string, string>> = {
  KeyQ: "й",
  KeyW: "ц",
  KeyE: "у",
  KeyR: "к",
  KeyT: "е",
  KeyY: "н",
  KeyU: "г",
  KeyI: "ш",
  KeyO: "щ",
  KeyP: "з",
  BracketLeft: "х",
  BracketRight: "ъ",
  KeyA: "ф",
  KeyS: "ы",
  KeyD: "в",
  KeyF: "а",
  KeyG: "п",
  KeyH: "р",
  KeyJ: "о",
  KeyK: "л",
  KeyL: "д",
  Semicolon: "ж",
  Quote: "э",
  KeyZ: "я",
  KeyX: "ч",
  KeyC: "с",
  KeyV: "м",
  KeyB: "и",
  KeyN: "т",
  KeyM: "ь",
  Comma: "б",
  Period: "ю",
  Backquote: "е",
};

/**
 * The letter a key press types in the game's language, or `null`. With the system keyboard in
 * the other language, the physical key decides, so a player need not switch layouts.
 */
export function letterForKey(language: WordHuntLanguage, key: string, code: string): string | null {
  const typed = key.toLowerCase().replace("ё", "е");
  if (language === WordHuntLanguage.RUSSIAN) {
    if (/^[а-я]$/.test(typed)) return typed;
    return RUSSIAN_BY_CODE[code] ?? null;
  }
  if (/^[a-z]$/.test(typed)) return typed;
  const physical = /^Key([A-Z])$/.exec(code);
  return physical?.[1]?.toLowerCase() ?? null;
}
