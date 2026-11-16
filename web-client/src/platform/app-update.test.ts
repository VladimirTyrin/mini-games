import { describe, expect, it } from "vitest";

import {
  type ApkRelease,
  type FetchJson,
  type InstallResult,
  type RunningApp,
  type WebBundleInstaller,
  type WebRelease,
  checkForUpdates,
  downloadsUrl,
  parseApkRelease,
  parseWebRelease,
  planUpdate,
} from "./app-update";

const SHELL = "0123456789abcdef";
const OTHER_SHELL = "fedcba9876543210";
const SHA256 = "a".repeat(64);
const running: RunningApp = { build: 40, nativeFingerprint: SHELL };

function webRelease(
  build: number,
  nativeFingerprint = SHELL,
  zipUrl = `https://example.org/downloads/web-${build}.zip`,
): WebRelease {
  return { build, versionName: `2.0.0 (${build})`, nativeFingerprint, zipUrl, sha256: SHA256 };
}

function apkRelease(build: number): ApkRelease {
  return {
    build,
    versionName: `2.0.0 (${build})`,
    downloadUrl: "https://example.org/downloads/mini-games.apk",
  };
}

describe("downloadsUrl", () => {
  it("points at the downloads of the game server", () => {
    expect(downloadsUrl("wss://example.org/ws", "web.json")?.toString()).toBe(
      "https://example.org/downloads/web.json",
    );
    expect(downloadsUrl("ws://10.0.0.5:8080/ws?x=1", "android.json")?.toString()).toBe(
      "http://10.0.0.5:8080/downloads/android.json",
    );
  });

  it("refuses addresses that are not WebSocket URLs", () => {
    expect(downloadsUrl("not a url", "web.json")).toBeNull();
    expect(downloadsUrl("https://example.org/", "web.json")).toBeNull();
  });
});

describe("release files", () => {
  const webJsonUrl = new URL("https://example.org/downloads/web.json");

  it("resolve file names next to the release file", () => {
    const web = {
      build: 42,
      versionName: "2.0.0 (42)",
      nativeFingerprint: SHELL,
      zip: "web-42.zip",
      sha256: SHA256,
    };
    expect(parseWebRelease(web, webJsonUrl)).toEqual(webRelease(42));
    const apk = { build: 42, versionName: "2.0.0 (42)", apk: "mini-games.apk" };
    expect(parseApkRelease(apk, new URL("https://example.org/downloads/android.json"))).toEqual(
      apkRelease(42),
    );
  });

  it("reject malformed content", () => {
    const web = {
      build: 42,
      versionName: "2.0.0 (42)",
      nativeFingerprint: SHELL,
      zip: "web-42.zip",
      sha256: SHA256,
    };
    expect(parseWebRelease({ ...web, build: "42" }, webJsonUrl)).toBeNull();
    expect(parseWebRelease({ ...web, sha256: "not a digest" }, webJsonUrl)).toBeNull();
    expect(parseWebRelease("<html>", webJsonUrl)).toBeNull();
    expect(parseApkRelease({ build: 1.5, versionName: "x", apk: "a.apk" }, webJsonUrl)).toBeNull();
  });
});

describe("planUpdate", () => {
  it("installs a newer web bundle built for the running shell, silently", () => {
    expect(planUpdate(running, webRelease(42), apkRelease(42))).toEqual({
      kind: "install-web-bundle",
      release: webRelease(42),
    });
  });

  it("offers the APK when the newer release needs another shell", () => {
    expect(planUpdate(running, webRelease(42, OTHER_SHELL), apkRelease(42))).toEqual({
      kind: "offer-apk",
      update: { versionName: "2.0.0 (42)", downloadUrl: "https://example.org/downloads/mini-games.apk" },
    });
  });

  it("offers the APK when the server publishes no web bundle (older server)", () => {
    expect(planUpdate(running, null, apkRelease(42)).kind).toBe("offer-apk");
  });

  it("offers the APK when the bundle would come over plain http", () => {
    const overHttp = webRelease(42, SHELL, "http://10.0.0.5/downloads/web-42.zip");
    expect(planUpdate(running, overHttp, apkRelease(42)).kind).toBe("offer-apk");
  });

  it("never matches a bundle without a known shell", () => {
    const unknownShell: RunningApp = { build: 40, nativeFingerprint: "" };
    expect(planUpdate(unknownShell, webRelease(42, ""), apkRelease(42)).kind).toBe("offer-apk");
  });

  it("does nothing when the running build is current", () => {
    expect(planUpdate(running, webRelease(40), apkRelease(40))).toEqual({ kind: "up-to-date" });
    expect(planUpdate(running, webRelease(39, OTHER_SHELL), apkRelease(39))).toEqual({ kind: "up-to-date" });
    expect(planUpdate(running, null, null)).toEqual({ kind: "up-to-date" });
  });

  it("compares with the running bundle, which may be newer than the shell after an OTA update", () => {
    const updatedOverTheAir: RunningApp = { build: 45, nativeFingerprint: SHELL };
    expect(planUpdate(updatedOverTheAir, webRelease(45), apkRelease(45))).toEqual({ kind: "up-to-date" });
  });
});

describe("checkForUpdates", () => {
  const files: Record<string, unknown> = {
    "https://example.org/downloads/web.json": {
      build: 42,
      versionName: "2.0.0 (42)",
      nativeFingerprint: SHELL,
      zip: "web-42.zip",
      sha256: SHA256,
    },
    "https://example.org/downloads/android.json": {
      build: 42,
      versionName: "2.0.0 (42)",
      apk: "mini-games.apk",
    },
  };
  const serve =
    (published: Record<string, unknown>): FetchJson =>
    (url) =>
      url.toString() in published
        ? Promise.resolve(published[url.toString()])
        : Promise.reject(new Error("404"));

  function recordingInstaller(
    result: InstallResult = "scheduled",
  ): WebBundleInstaller & { installed: WebRelease[] } {
    const installed: WebRelease[] = [];
    return {
      installed,
      installLater: (release) => {
        installed.push(release);
        return Promise.resolve(result);
      },
    };
  }

  it("hands a matching web bundle to the installer", async () => {
    const installer = recordingInstaller();
    const result = await checkForUpdates("wss://example.org/ws", running, installer, serve(files));
    expect(result).toEqual({ kind: "web-bundle-ready", versionName: "2.0.0 (42)" });
    expect(installer.installed).toEqual([webRelease(42)]);
  });

  it("reports nothing to do for a build that already failed to start here", async () => {
    const installer = recordingInstaller("known-broken");
    const result = await checkForUpdates("wss://example.org/ws", running, installer, serve(files));
    expect(result).toEqual({ kind: "up-to-date" });
  });

  it("falls back to the APK check when web.json is missing", async () => {
    const installer = recordingInstaller();
    const onlyApk = {
      "https://example.org/downloads/android.json": files["https://example.org/downloads/android.json"],
    };
    const result = await checkForUpdates("wss://example.org/ws", running, installer, serve(onlyApk));
    expect(result.kind).toBe("apk-available");
    expect(installer.installed).toEqual([]);
  });

  it("tells when nothing could be read", async () => {
    const result = await checkForUpdates("wss://example.org/ws", running, recordingInstaller(), serve({}));
    expect(result).toEqual({ kind: "no-release-info" });
  });
});
