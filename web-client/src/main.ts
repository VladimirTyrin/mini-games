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
}

boot().catch((error: unknown) => {
  document.body.textContent = `Failed to start: ${errorMessage(error)}`;
});
