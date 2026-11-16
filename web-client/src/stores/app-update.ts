import { defineStore } from "pinia";
import { shallowRef, watch } from "vue";

import {
  type AvailableUpdate,
  type UpdateCheckResult,
  checkForUpdates,
  downloadUpdate,
} from "@/platform/app-update";
import { nativeWebBundles } from "@/platform/web-bundle";

import { useMatchStore } from "./match";
import { usePrefsStore } from "./prefs";

export type UpdateCheckReport = UpdateCheckResult | { readonly kind: "check-failed" };

/**
 * Android only. A newer web bundle for this shell is downloaded silently and switched to when
 * the app next goes to the background (never during a match); only a changed shell makes
 * `available` offer an APK.
 */
export const useAppUpdateStore = defineStore("app-update", () => {
  const prefs = usePrefsStore();
  const match = useMatchStore();
  const available = shallowRef<AvailableUpdate | null>(null);
  /** Version name of a downloaded web bundle that is waiting for the switch. */
  const webBundleReady = shallowRef<string | null>(null);
  let running: Promise<UpdateCheckReport> | null = null;

  async function runCheck(): Promise<UpdateCheckReport> {
    const bundles = await nativeWebBundles();
    if (!bundles) return { kind: "up-to-date" };
    try {
      const result = await checkForUpdates(
        prefs.serverUrl,
        { build: __APP_BUILD__, nativeFingerprint: __NATIVE_FINGERPRINT__ },
        bundles,
      );
      available.value = result.kind === "apk-available" ? result.update : null;
      if (result.kind === "web-bundle-ready") {
        webBundleReady.value = result.versionName;
        await bundles.holdSwitch(match.controller !== null);
      }
      return result;
    } catch {
      // Offline, or the download failed: try again on the next check.
      return { kind: "check-failed" };
    }
  }

  function check(): Promise<UpdateCheckReport> {
    if (!__NATIVE_APP__) return Promise.resolve({ kind: "up-to-date" });
    if (!running) {
      running = runCheck().finally(() => {
        running = null;
      });
    }
    return running;
  }

  async function install(): Promise<void> {
    if (available.value) await downloadUpdate(available.value);
  }

  if (__NATIVE_APP__) {
    watch(
      () => match.controller !== null,
      async (inMatch) => {
        if (!webBundleReady.value) return;
        try {
          await (await nativeWebBundles())?.holdSwitch(inMatch);
        } catch {
          // The plugin then switches on the next background as usual.
        }
      },
    );
  }

  return { available, webBundleReady, check, install };
});
