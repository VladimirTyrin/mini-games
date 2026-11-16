import { readFileSync } from "node:fs";
import { fileURLToPath, URL } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

function workspaceVersion(): string {
  const cargo = readFileSync(new URL("../Cargo.toml", import.meta.url), "utf-8");
  return /\[workspace\.package\][^[]*?version\s*=\s*"([^"]+)"/s.exec(cargo)?.[1] ?? "0.0.0";
}

export default defineConfig({
  // Relative base: the same build works on the server root, GitHub Pages and inside Capacitor.
  base: process.env.VITE_BASE_PATH ?? "./",
  plugins: [vue(), tailwindcss()],
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  define: {
    __APP_VERSION__: JSON.stringify(workspaceVersion()),
    // A literal, so the web build tree-shakes the Capacitor plugins away entirely.
    __NATIVE_APP__: JSON.stringify(process.env.VITE_PLATFORM === "android"),
    // Android builds compare it with the server's android.json to offer updates.
    __APP_BUILD__: JSON.stringify(Number(process.env.MG_BUILD_NUMBER ?? 0)),
  },
  build: { target: "es2022", assetsInlineLimit: 0 },
  worker: { format: "es" },
  server: {
    proxy: { "/ws": { target: "ws://127.0.0.1:5000", ws: true } },
  },
});
