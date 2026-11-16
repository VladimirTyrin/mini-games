// Over-the-air web bundles through @capgo/capacitor-updater in manual mode (no Capgo cloud; see
// the plugin config in capacitor.config.ts). The plugin keeps downloaded bundles on the device,
// switches to the `next` one when the app goes to the background, and rolls a bundle back when
// it does not call `notifyAppReady` in time after starting.
import type { CapacitorUpdaterPlugin } from "@capgo/capacitor-updater";

import type { InstallResult, WebBundleInstaller, WebRelease } from "./app-update";

export type UpdaterPlugin = Pick<
  CapacitorUpdaterPlugin,
  "list" | "download" | "next" | "setMultiDelay" | "cancelDelay"
>;

// A Capacitor plugin object is a proxy that turns every property, `then` included, into a native
// call. Never return it from an async function or resolve a promise with it: the promise would
// call `plugin.then(...)` and never settle. Only plain wrappers around it cross async boundaries.

/**
 * Tells the plugin this bundle started; without it the plugin restores the previous bundle.
 * Called once the app has mounted, so a bundle that cannot even start is rolled back.
 */
export async function confirmBundleStarted(): Promise<void> {
  if (!__NATIVE_APP__) return;
  const { CapacitorUpdater } = await import("@capgo/capacitor-updater");
  await CapacitorUpdater.notifyAppReady();
}

export interface PluginWebBundles extends WebBundleInstaller {
  /**
   * While held, the plugin does not switch to a scheduled bundle when the app goes to the
   * background, so a player who leaves mid-match comes back to the same match. The `kill`
   * condition is also cleared by the next cold start: a hold never outlives the process.
   */
  holdSwitch(held: boolean): Promise<void>;
}

/** Reuses an earlier download of the same build; a build that failed to start is not retried. */
export function pluginWebBundles(plugin: UpdaterPlugin): PluginWebBundles {
  return {
    async installLater(release: WebRelease): Promise<InstallResult> {
      const version = String(release.build);
      const { bundles } = await plugin.list();
      const earlier = bundles.find((bundle) => bundle.version === version && bundle.status !== "deleted");
      if (earlier?.status === "error") return "known-broken";
      const bundle =
        earlier && earlier.status !== "downloading"
          ? earlier
          : await plugin.download({ url: release.zipUrl, version, checksum: release.sha256 });
      await plugin.next({ id: bundle.id });
      return "scheduled";
    },
    async holdSwitch(held: boolean): Promise<void> {
      if (held) await plugin.setMultiDelay({ delayConditions: [{ kind: "kill" }] });
      else await plugin.cancelDelay();
    },
  };
}

export async function nativeWebBundles(): Promise<PluginWebBundles | null> {
  if (!__NATIVE_APP__) return null;
  const { CapacitorUpdater } = await import("@capgo/capacitor-updater");
  return pluginWebBundles(CapacitorUpdater);
}

/** The shell's own build (versionCode = major * 1 000 000 + build, see android/app/build.gradle). */
export async function nativeShellBuild(): Promise<number | null> {
  if (!__NATIVE_APP__) return null;
  const { App } = await import("@capacitor/app");
  const versionCode = Number((await App.getInfo()).build);
  return Number.isSafeInteger(versionCode) ? versionCode % 1_000_000 : null;
}
