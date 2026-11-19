import { locale } from "@/i18n";

const DAY_MS = 24 * 60 * 60 * 1000;

/**
 * Days since 1970-01-01 on the player's calendar: the word of the day changes at local
 * midnight, like any daily puzzle.
 */
export function localDayNumber(now: Date = new Date()): number {
  return Math.floor(Date.UTC(now.getFullYear(), now.getMonth(), now.getDate()) / DAY_MS);
}

/** "16 Nov" / "16 нояб." for a day number. */
export function formatDay(day: number): string {
  return new Date(day * DAY_MS).toLocaleDateString(locale.value, {
    day: "numeric",
    month: "short",
    timeZone: "UTC",
  });
}
