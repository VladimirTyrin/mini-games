/**
 * Browser download to the usual downloads folder. The anchor must be in the DOM and the URL
 * revoked later for Safari. The app's WebView ignores download links: it keeps files in its own
 * folder instead (`saved-replays.ts`).
 */
export function downloadInBrowser(bytes: Uint8Array, fileName: string): void {
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
