import { locale } from "@/i18n";

/** One digit after the point, as the language writes it: "12.3" / "12,3". */
export function formatOneDecimal(value: number): string {
  return new Intl.NumberFormat(locale.value, { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(
    value,
  );
}
