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
  createApp(App).use(createPinia()).use(router).mount("#app");
  await setUpNativeApp();
  if (!__NATIVE_APP__) {
    // Offline support and silent updates: a new version is fetched in the background and used
    // from the next start.
    const { registerSW } = await import("virtual:pwa-register");
    void registerSW({ immediate: true });
  }
}

boot().catch((error: unknown) => {
  document.body.textContent = `Failed to start: ${errorMessage(error)}`;
});
