import type { RegisterSWOptions } from "vite-plugin-pwa/types";
import { watch } from "vue";

export type RegisterServiceWorker = (options: RegisterSWOptions) => (reloadPage?: boolean) => Promise<void>;

/** While the app is open; returning to it checks at once. */
export const UPDATE_CHECK_INTERVAL_MS = 30 * 60 * 1000;

/**
 * Web build only. iOS resumes a home-screen app from memory without navigating, and the browser
 * only re-checks the service worker on a navigation, so the app checks on every return to it and
 * every half hour. A new version is applied (the page reloads) right away, or as soon as
 * `holdsUpdates` lets go: never mid-match or in an online lobby.
 */
export function startWebUpdates(register: RegisterServiceWorker, holdsUpdates: () => boolean): void {
  let updateWaiting = false;

  function applyWhenIdle(): void {
    if (updateWaiting && !holdsUpdates()) void applyUpdate(true);
  }

  const applyUpdate = register({
    immediate: true,
    onNeedRefresh() {
      updateWaiting = true;
      applyWhenIdle();
    },
    onRegisteredSW(_url, registration) {
      if (!registration) return;
      const check = (): void => {
        if (document.visibilityState !== "visible") return;
        // Offline: the next return or tick tries again.
        registration.update().catch(() => undefined);
      };
      document.addEventListener("visibilitychange", check);
      setInterval(check, UPDATE_CHECK_INTERVAL_MS);
    },
  });

  watch(holdsUpdates, applyWhenIdle);
}
