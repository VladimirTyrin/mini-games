import { fileURLToPath, URL } from "node:url";

import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [vue()],
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  define: { __APP_VERSION__: JSON.stringify("test"), __NATIVE_APP__: "false" },
  test: { environment: "happy-dom", include: ["src/**/*.test.ts"] },
});
