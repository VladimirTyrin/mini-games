import type { BundleInfo, BundleStatus, DownloadOptions } from "@capgo/capacitor-updater";
import { describe, expect, it } from "vitest";

import type { WebRelease } from "./app-update";
import { type UpdaterPlugin, pluginWebBundles } from "./web-bundle";

const release: WebRelease = {
  build: 42,
  versionName: "2.0.0 (42)",
  nativeFingerprint: "0123456789abcdef",
  zipUrl: "https://example.org/downloads/web-42.zip",
  sha256: "b".repeat(64),
};

function bundle(id: string, version: string, status: BundleStatus): BundleInfo {
  return { id, version, status, downloaded: "", checksum: "" };
}

/** The plugin's bundle list and delay state, as far as the app uses them. */
class FakeUpdater implements UpdaterPlugin {
  readonly downloads: DownloadOptions[] = [];
  nextBundleId: string | null = null;
  activeBundleId: string | null = null;
  switchHeld = false;

  constructor(readonly bundles: BundleInfo[]) {}

  list: UpdaterPlugin["list"] = () => Promise.resolve({ bundles: [...this.bundles] });

  download: UpdaterPlugin["download"] = (options) => {
    this.downloads.push(options);
    const downloaded = bundle(`id-${this.bundles.length}`, options.version, "pending");
    this.bundles.push(downloaded);
    return Promise.resolve(downloaded);
  };

  next: UpdaterPlugin["next"] = ({ id }) => {
    this.nextBundleId = id;
    const found = this.bundles.find((candidate) => candidate.id === id);
    return found ? Promise.resolve(found) : Promise.reject(new Error(`no bundle ${id}`));
  };

  set: UpdaterPlugin["set"] = ({ id }) => {
    this.activeBundleId = id;
    return Promise.resolve();
  };

  setMultiDelay: UpdaterPlugin["setMultiDelay"] = () => {
    this.switchHeld = true;
    return Promise.resolve();
  };

  cancelDelay: UpdaterPlugin["cancelDelay"] = () => {
    this.switchHeld = false;
    return Promise.resolve();
  };
}

describe("pluginWebBundles", () => {
  it("downloads the zip with its checksum and makes it the next bundle", async () => {
    const plugin = new FakeUpdater([bundle("builtin", "builtin", "success")]);
    expect(await pluginWebBundles(plugin).installLater(release)).toBe("scheduled");
    expect(plugin.downloads).toEqual([{ url: release.zipUrl, version: "42", checksum: release.sha256 }]);
    expect(plugin.nextBundleId).toBe("id-1");
  });

  it("reuses a bundle downloaded by an earlier check", async () => {
    const plugin = new FakeUpdater([bundle("builtin", "builtin", "success"), bundle("b42", "42", "pending")]);
    expect(await pluginWebBundles(plugin).installLater(release)).toBe("scheduled");
    expect(plugin.downloads).toEqual([]);
    expect(plugin.nextBundleId).toBe("b42");
  });

  it("does not retry a build that failed to start", async () => {
    const plugin = new FakeUpdater([bundle("builtin", "builtin", "success"), bundle("b42", "42", "error")]);
    expect(await pluginWebBundles(plugin).installLater(release)).toBe("known-broken");
    expect(plugin.downloads).toEqual([]);
    expect(plugin.nextBundleId).toBeNull();
  });

  it("can be returned from an async function although the plugin is a Capacitor proxy", async () => {
    // Like Capacitor's plugin proxy: any unknown property, `then` included, is a native call.
    const fake = new FakeUpdater([bundle("builtin", "builtin", "success")]);
    const proxy = new Proxy(fake, {
      get: (target, property) =>
        property in target ? Reflect.get(target, property) : () => new Promise<never>(() => {}),
    });
    const bundles = await (async () => pluginWebBundles(proxy))();
    expect(await bundles.installLater(release)).toBe("scheduled");
  });

  it("holds and releases the switch", async () => {
    const plugin = new FakeUpdater([]);
    const bundles = pluginWebBundles(plugin);
    await bundles.holdSwitch(true);
    expect(plugin.switchHeld).toBe(true);
    await bundles.holdSwitch(false);
    expect(plugin.switchHeld).toBe(false);
  });

  it("switches to the scheduled bundle at once when asked", async () => {
    const plugin = new FakeUpdater([]);
    const bundles = pluginWebBundles(plugin);
    await bundles.switchNow();
    expect(plugin.activeBundleId).toBeNull();
    await bundles.installLater(release);
    await bundles.switchNow();
    expect(plugin.activeBundleId).toBe(plugin.nextBundleId);
  });
});
