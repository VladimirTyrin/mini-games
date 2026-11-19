import { ref, watch } from "vue";

import { storage } from "@/platform/storage";

export const LOCALES = ["en", "ru"] as const;
export type Locale = (typeof LOCALES)[number];

/** Read by the screen tools too (`tools/screens`), which set it before the app loads. */
export const LOCALE_STORAGE_KEY = "mini-games.locale";

function isLocale(value: string | null): value is Locale {
  return (LOCALES as readonly (string | null)[]).includes(value);
}

/** The device's language: Russian if it comes first among Russian and the rest, else English. */
export function deviceLocale(languages: readonly string[]): Locale {
  const first = languages.find((language) => language.trim() !== "");
  return first?.toLowerCase().startsWith("ru") ? "ru" : "en";
}

function startingLocale(): Locale {
  const saved = storage.get(LOCALE_STORAGE_KEY);
  if (isLocale(saved)) return saved;
  return deviceLocale(typeof navigator === "undefined" ? [] : navigator.languages);
}

/** The language the interface speaks. Change it with `setLocale`. */
export const locale = ref<Locale>(startingLocale());

/** Whether the player picked the language themself (not taken from the device). */
export function hasChosenLocale(): boolean {
  return isLocale(storage.get(LOCALE_STORAGE_KEY));
}

export function setLocale(next: Locale): void {
  storage.set(LOCALE_STORAGE_KEY, next);
  locale.value = next;
}

const listeners: ((next: Locale) => void)[] = [];

/** Runs `listener` now and whenever the language changes (native per-app locale, the worker). */
export function onLocale(listener: (next: Locale) => void): void {
  listeners.push(listener);
  listener(locale.value);
}

watch(
  locale,
  (next) => {
    if (typeof document !== "undefined") document.documentElement.lang = next;
    for (const listener of listeners) listener(next);
  },
  { immediate: true },
);
