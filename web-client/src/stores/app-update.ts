import { defineStore } from "pinia";
import { shallowRef } from "vue";

import { type AvailableUpdate, downloadUpdate, findUpdate } from "@/platform/app-update";

import { usePrefsStore } from "./prefs";

/** Android only: whether the server publishes a newer APK than the one running. */
export const useAppUpdateStore = defineStore("app-update", () => {
  const prefs = usePrefsStore();
  const available = shallowRef<AvailableUpdate | null>(null);

  async function check(): Promise<void> {
    if (!__NATIVE_APP__) return;
    try {
      available.value = await findUpdate(prefs.serverUrl, __APP_BUILD__);
    } catch {
      // Offline or an old server without android.json: simply no update offer.
      available.value = null;
    }
  }

  async function install(): Promise<void> {
    if (available.value) await downloadUpdate(available.value);
  }

  return { available, check, install };
});
