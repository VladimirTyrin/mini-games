import { isNativeApp } from "./native-app";

/** What happened to a file the player asked to save. */
export type SaveOutcome =
  /** The browser downloaded it (to its usual downloads folder). */
  | "downloaded"
  /** The app handed it to the system share sheet. */
  | "shared"
  /** The player closed the share sheet. */
  | "cancelled";

/** Browser download. The anchor must be in the DOM and the URL revoked later for Safari. */
function downloadInBrowser(bytes: Uint8Array, fileName: string): void {
  const blob = new Blob([bytes.slice().buffer], { type: "application/octet-stream" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

function toBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

/**
 * The app's WebView ignores download links, so the file goes to the cache and then to the
 * system share sheet, where the player saves it to Files or Drive or sends it to someone.
 */
async function shareFromApp(bytes: Uint8Array, fileName: string): Promise<SaveOutcome> {
  const [{ Directory, Filesystem }, { Share }] = await Promise.all([
    import("@capacitor/filesystem"),
    import("@capacitor/share"),
  ]);
  const written = await Filesystem.writeFile({
    path: fileName,
    data: toBase64(bytes),
    directory: Directory.Cache,
  });
  try {
    await Share.share({ title: fileName, files: [written.uri] });
    return "shared";
  } catch {
    // The plugin rejects when the sheet is dismissed; there is nothing else it reports.
    return "cancelled";
  }
}

export async function saveBytes(bytes: Uint8Array, fileName: string): Promise<SaveOutcome> {
  if (isNativeApp) return shareFromApp(bytes, fileName);
  downloadInBrowser(bytes, fileName);
  return "downloaded";
}
