import "./main.css";

import { createPinia } from "pinia";
import { createApp } from "vue";

import App from "./App.vue";
import { setUpNativeApp } from "./platform/native-app";
import { hasChosenLocale, onLocale, t } from "./i18n";
import { setNativeAppLocale } from "./platform/app-locale";
import { keepLocaleForWorker } from "./platform/worker-locale";
import { router } from "./router";
import { loadCore } from "./wasm/core";
import { errorMessage } from "./util/error-message";

async function boot(): Promise<void> {
  await loadCore();
  // Push notifications are worded on the device: by Android in the app's language, by the
  // service worker in the one it reads from IndexedDB.
  onLocale((locale) => {
    void setNativeAppLocale(hasChosenLocale() ? locale : null);
    keepLocaleForWorker(locale);
  });
  const pinia = createPinia();
  createApp(App).use(pinia).use(router).mount("#app");
  await setUpNativeApp();
  const [{ useNotificationsStore }, { useFriendsStore }, links] = await Promise.all([
    import("./stores/notifications"),
    import("./stores/friends"),
    import("./platform/friend-links"),
  ]);
  await useNotificationsStore(pinia).listen(router);
  const friends = useFriendsStore(pinia);
  const openFriendLink = (code: string): void => {
    void friends.useLink(code);
    void router.push("/friends");
  };
  const code = links.takeFriendCodeFromAddress();
  if (code) openFriendLink(code);
  await links.onFriendLinkOpened(openFriendLink);
  // A replay file opened with the Android app (Open with, Share).
  const [{ onReplayFileOpened }, { useMatchStore: matchStore }, { useToastStore }] = await Promise.all([
    import("./platform/incoming-replays"),
    import("./stores/match"),
    import("./stores/toasts"),
  ]);
  await onReplayFileOpened((bytes) => {
    const match = matchStore(pinia);
    // A match being played is not dropped for a replay: it can wait.
    if (match.isPlaying && match.controller?.source.kind !== "replay") {
      useToastStore(pinia).show(t.app.leaveMatchToWatchReplay);
      return;
    }
    void match.importReplay(bytes);
  });
  if (!__NATIVE_APP__) {
    // Offline support; new versions are applied outside matches (see web-updates.ts).
    const [{ registerSW }, { startWebUpdates }, { useMatchStore }] = await Promise.all([
      import("virtual:pwa-register"),
      import("./platform/web-updates"),
      import("./stores/match"),
    ]);
    const match = useMatchStore(pinia);
    startWebUpdates(registerSW, () => match.holdsUpdates);
  }
}

boot().catch((error: unknown) => {
  document.body.textContent = t.app.failedToStart(errorMessage(error));
});
