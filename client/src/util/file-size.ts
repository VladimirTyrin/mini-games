import { t } from "@/i18n";

import { formatOneDecimal } from "./format-decimal";

const KIB = 1024;

/** "850 B", "12.3 KB", "1.2 MB". */
export function formatFileSize(bytes: number): string {
  if (bytes < KIB) return t.app.units.bytes(bytes);
  if (bytes < KIB * KIB) return t.app.units.kilobytes(formatOneDecimal(bytes / KIB));
  return t.app.units.megabytes(formatOneDecimal(bytes / (KIB * KIB)));
}
