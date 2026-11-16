// Android updates, published next to the game server under /downloads/ by deploy.sh:
// - web.json + web-<build>.zip: the web bundle, installed over the air into a matching shell;
// - android.json + mini-games.apk: the whole app, offered only when the web bundle cannot be.

/** A newer APK the player can download and install. */
export interface AvailableUpdate {
  readonly versionName: string;
  readonly downloadUrl: string;
}

/** `web.json`: the web bundle of the latest release. */
export interface WebRelease {
  readonly build: number;
  readonly versionName: string;
  readonly nativeFingerprint: string;
  readonly zipUrl: string;
  /** Hex SHA-256 of the zip; the updater plugin refuses a download that does not match. */
  readonly sha256: string;
}

/** `android.json`: the APK of the latest release. */
export interface ApkRelease {
  readonly build: number;
  readonly versionName: string;
  readonly downloadUrl: string;
}

/** The bundle that is running and the shell it runs in. */
export interface RunningApp {
  readonly build: number;
  readonly nativeFingerprint: string;
}

export type UpdatePlan =
  | { readonly kind: "up-to-date" }
  | { readonly kind: "install-web-bundle"; readonly release: WebRelease }
  | { readonly kind: "offer-apk"; readonly update: AvailableUpdate };

export type UpdateCheckResult =
  | { readonly kind: "up-to-date" }
  /** Neither release file could be read: offline, or a server that publishes no app. */
  | { readonly kind: "no-release-info" }
  | { readonly kind: "web-bundle-ready"; readonly versionName: string }
  | { readonly kind: "apk-available"; readonly update: AvailableUpdate };

/** `known-broken`: this build already failed to start on the device and is not tried again. */
export type InstallResult = "scheduled" | "known-broken";

/** Downloads a web bundle and switches to it the next time that cannot interrupt the player. */
export interface WebBundleInstaller {
  installLater(release: WebRelease): Promise<InstallResult>;
}

export type FetchJson = (url: URL) => Promise<unknown>;

/** `wss://host/ws` → `https://host/downloads/<fileName>` (plain `ws://` → `http://`). */
export function downloadsUrl(serverUrl: string, fileName: string): URL | null {
  try {
    const url = new URL(serverUrl);
    if (url.protocol !== "ws:" && url.protocol !== "wss:") return null;
    url.protocol = url.protocol === "wss:" ? "https:" : "http:";
    url.pathname = `/downloads/${fileName}`;
    url.search = "";
    return url;
  } catch {
    return null;
  }
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : null;
}

function isBuildNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0;
}

export function parseWebRelease(json: unknown, webJsonUrl: URL): WebRelease | null {
  const record = asRecord(json);
  if (
    !record ||
    !isBuildNumber(record.build) ||
    typeof record.versionName !== "string" ||
    typeof record.nativeFingerprint !== "string" ||
    typeof record.zip !== "string" ||
    typeof record.sha256 !== "string" ||
    !/^[0-9a-f]{64}$/.test(record.sha256)
  ) {
    return null;
  }
  return {
    build: record.build,
    versionName: record.versionName,
    nativeFingerprint: record.nativeFingerprint,
    zipUrl: new URL(record.zip, webJsonUrl).toString(),
    sha256: record.sha256,
  };
}

export function parseApkRelease(json: unknown, androidJsonUrl: URL): ApkRelease | null {
  const record = asRecord(json);
  if (
    !record ||
    !isBuildNumber(record.build) ||
    typeof record.versionName !== "string" ||
    typeof record.apk !== "string"
  ) {
    return null;
  }
  return {
    build: record.build,
    versionName: record.versionName,
    downloadUrl: new URL(record.apk, androidJsonUrl).toString(),
  };
}

/**
 * A newer web bundle built for this very shell is installed silently. Only over https: the
 * bundle is code, and the plugin refuses plain-http downloads (`httpsOnly`). In every other
 * case (other shell, plain http, no web.json on an older server) a newer APK is offered.
 */
export function planUpdate(running: RunningApp, web: WebRelease | null, apk: ApkRelease | null): UpdatePlan {
  if (
    web &&
    web.build > running.build &&
    running.nativeFingerprint !== "" &&
    web.nativeFingerprint === running.nativeFingerprint &&
    new URL(web.zipUrl).protocol === "https:"
  ) {
    return { kind: "install-web-bundle", release: web };
  }
  if (apk && apk.build > running.build) {
    return { kind: "offer-apk", update: { versionName: apk.versionName, downloadUrl: apk.downloadUrl } };
  }
  return { kind: "up-to-date" };
}

export const fetchJsonNoCache: FetchJson = async (url) => {
  const response = await fetch(url, { cache: "no-store" });
  if (!response.ok) throw new Error(`${url.toString()}: HTTP ${response.status}`);
  return response.json();
};

async function fetchRelease<T>(
  fetchJson: FetchJson,
  url: URL | null,
  parse: (json: unknown, url: URL) => T | null,
): Promise<T | null> {
  if (!url) return null;
  try {
    return parse(await fetchJson(url), url);
  } catch {
    // Offline, or a server that does not publish this file: nothing to offer from it.
    return null;
  }
}

/** Reads both release files from the server the app talks to and acts on {@link planUpdate}. */
export async function checkForUpdates(
  serverUrl: string,
  running: RunningApp,
  installer: WebBundleInstaller,
  fetchJson: FetchJson = fetchJsonNoCache,
): Promise<UpdateCheckResult> {
  const [web, apk] = await Promise.all([
    fetchRelease(fetchJson, downloadsUrl(serverUrl, "web.json"), parseWebRelease),
    fetchRelease(fetchJson, downloadsUrl(serverUrl, "android.json"), parseApkRelease),
  ]);
  if (!web && !apk) return { kind: "no-release-info" };
  const plan = planUpdate(running, web, apk);
  switch (plan.kind) {
    case "install-web-bundle":
      if ((await installer.installLater(plan.release)) === "known-broken") return { kind: "up-to-date" };
      return { kind: "web-bundle-ready", versionName: plan.release.versionName };
    case "offer-apk":
      return { kind: "apk-available", update: plan.update };
    case "up-to-date":
      return plan;
  }
}

/** Hands the APK to the system browser, which downloads it and offers to install. */
export async function downloadUpdate(update: AvailableUpdate): Promise<void> {
  if (!__NATIVE_APP__) return;
  const { Browser } = await import("@capacitor/browser");
  await Browser.open({ url: update.downloadUrl });
}
