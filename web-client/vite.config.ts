import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import { readFileSync } from "fs";
import { parse } from "toml";
import { resolve } from "path";

function getVersionFromCargoToml(): string {
  const cargoTomlPath = resolve(__dirname, "../Cargo.toml");
  const cargoToml = readFileSync(cargoTomlPath, "utf-8");
  const parsed = parse(cargoToml) as { workspace?: { package?: { version?: string } } };
  return parsed.workspace?.package?.version ?? "0.0.0";
}

export default defineConfig({
  base: process.env.VITE_BASE_PATH ?? "/ui/",
  plugins: [vue(), tailwindcss()],
  define: {
    __APP_VERSION__: JSON.stringify(getVersionFromCargoToml()),
  },
  server: {
    proxy: {
      "/ws": {
        target: "ws://localhost:5000",
        ws: true,
        changeOrigin: true,
      },
    },
  },
});
