import "./main.css";

import { createPinia } from "pinia";
import { createApp } from "vue";

import App from "./App.vue";
import { setUpNativeApp } from "./platform/native-app";
import { router } from "./router";
import { loadCore } from "./wasm/core";
import { errorMessage } from "./util/error-message";

async function boot(): Promise<void> {
  await loadCore();
  const pinia = createPinia();
  createApp(App).use(pinia).use(router).mount("#app");
  await setUpNativeApp();
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
  document.body.textContent = `Failed to start: ${errorMessage(error)}`;
});
