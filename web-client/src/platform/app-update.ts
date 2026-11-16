import { isNativeApp } from "./native-app";

/** A newer APK published next to the game server (`/downloads/android.json`). */
export interface AvailableUpdate {
  readonly versionName: string;
  readonly downloadUrl: string;
}

interface PublishedBuild {
  build: number;
  versionName: string;
  apk: string;
}

/** `wss://host/ws` → `https://host/downloads/android.json` (plain `ws://` → `http://`). */
export function updateInfoUrl(serverUrl: string): URL | null {
  try {
    const url = new URL(serverUrl);
    if (url.protocol !== "ws:" && url.protocol !== "wss:") return null;
    url.protocol = url.protocol === "wss:" ? "https:" : "http:";
    url.pathname = "/downloads/android.json";
    url.search = "";
    return url;
  } catch {
    return null;
  }
}

function isPublishedBuild(value: unknown): value is PublishedBuild {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.build === "number" &&
    typeof candidate.versionName === "string" &&
    typeof candidate.apk === "string"
  );
}

/** `null` when this build is current or the server publishes nothing usable. */
export async function findUpdate(
  serverUrl: string,
  currentBuild: number,
  fetchJson: (url: URL) => Promise<unknown> = async (url) => (await fetch(url, { cache: "no-store" })).json(),
): Promise<AvailableUpdate | null> {
  const infoUrl = updateInfoUrl(serverUrl);
  if (!infoUrl) return null;
  const published = await fetchJson(infoUrl);
  if (!isPublishedBuild(published) || published.build <= currentBuild) return null;
  return { versionName: published.versionName, downloadUrl: new URL(published.apk, infoUrl).toString() };
}

/** Hands the APK to the system browser, which downloads it and offers to install. */
export async function downloadUpdate(update: AvailableUpdate): Promise<void> {
  if (!isNativeApp) return;
  const { Browser } = await import("@capacitor/browser");
  await Browser.open({ url: update.downloadUrl });
}
