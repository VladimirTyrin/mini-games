const KIB = 1024;

/** "850 B", "12.3 KB", "1.2 MB". */
export function formatFileSize(bytes: number): string {
  if (bytes < KIB) return `${bytes} B`;
  if (bytes < KIB * KIB) return `${(bytes / KIB).toFixed(1)} KB`;
  return `${(bytes / (KIB * KIB)).toFixed(1)} MB`;
}
