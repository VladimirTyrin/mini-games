import { en } from "./en";
import { type Locale, locale } from "./locale";
import type { Messages } from "./messages";
import { ru } from "./ru";

const dictionaries: Record<Locale, Messages> = { en, ru };

/** The dictionary of a language, for what must not follow the current one (the push worker). */
export function messagesFor(language: Locale): Messages {
  return dictionaries[language];
}

/**
 * The texts in the current language: `t.lobby.title`, `t.match.wins(3)`. Reading through it
 * depends on the language, so templates and computeds redraw when it changes; keep the path from
 * `t` (don't hold on to `t.lobby`) where the text must follow a switch.
 */
export const t: Messages = new Proxy({} as Messages, {
  get: (_target, area) => Reflect.get(dictionaries[locale.value], area),
});

export type { Locale } from "./locale";
export type { Messages } from "./messages";
export { LOCALES, locale, setLocale, hasChosenLocale, onLocale } from "./locale";
export { pluralEn, pluralRu } from "./plural";
export { formatDate, formatDateTime, formatNumber, formatTimeWithSeconds } from "./format";
