import { locale } from "./locale";

/** A whole number with the language's grouping: 12,345 / 12 345. */
export function formatNumber(value: number | bigint): string {
  return new Intl.NumberFormat(locale.value).format(value);
}

/** A date and time a player reads: "Nov 19, 2026, 14:05" / "19 нояб. 2026 г., 14:05". */
export function formatDateTime(date: Date): string {
  return new Intl.DateTimeFormat(locale.value, { dateStyle: "medium", timeStyle: "short" }).format(date);
}

/** A date alone: "Nov 19, 2026" / "19 нояб. 2026 г.". */
export function formatDate(date: Date): string {
  return new Intl.DateTimeFormat(locale.value, { dateStyle: "medium" }).format(date);
}
