import { fileURLToPath, URL } from "node:url";

import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [vue()],
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  define: {
    __APP_VERSION__: JSON.stringify("test"),
    __NATIVE_APP__: "false",
    __APP_BUILD__: "0",
    __NATIVE_FINGERPRINT__: JSON.stringify(""),
  },
  test: { environment: "happy-dom", include: ["src/**/*.test.ts", "build/**/*.test.mjs"] },
});
