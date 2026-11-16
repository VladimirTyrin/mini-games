import "./main.css";

import { createPinia } from "pinia";
import { createApp } from "vue";

import App from "./App.vue";
import { router } from "./router";
import { loadCore } from "./wasm/core";

async function boot(): Promise<void> {
  await loadCore();
  createApp(App).use(createPinia()).use(router).mount("#app");
}

boot().catch((error: unknown) => {
  document.body.textContent = `Failed to start: ${error instanceof Error ? error.message : String(error)}`;
});
