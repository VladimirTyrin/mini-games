import { locale } from "./locale";

/** A whole number with the language's grouping: 12,345 / 12 345. */
export function formatNumber(value: number | bigint): string {
  return new Intl.NumberFormat(locale.value).format(value);
}

/** The app's clock is 24-hour in every language (English included: 14:05, not 2:05 PM). */
const CLOCK = { hourCycle: "h23" } as const;

/** A date and time a player reads: "Nov 19, 2026, 14:05" / "19 нояб. 2026 г., 14:05". */
export function formatDateTime(date: Date): string {
  return new Intl.DateTimeFormat(locale.value, { dateStyle: "medium", timeStyle: "short", ...CLOCK }).format(
    date,
  );
}

/** A time to the second: "14:05:09". */
export function formatTimeWithSeconds(date: Date): string {
  return new Intl.DateTimeFormat(locale.value, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    ...CLOCK,
  }).format(date);
}

/** A date alone: "Nov 19, 2026" / "19 нояб. 2026 г.". */
export function formatDate(date: Date): string {
  return new Intl.DateTimeFormat(locale.value, { dateStyle: "medium" }).format(date);
}
