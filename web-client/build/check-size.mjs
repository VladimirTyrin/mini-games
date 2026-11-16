// Fails the build when the gzip size of the shipped bundle exceeds the budget.
import { gzipSync } from "node:zlib";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const assets = resolve(dirname(fileURLToPath(import.meta.url)), "../dist/assets");
const budgetsKiB = { js: 120, wasm: 250, css: 30 };
const totals = { js: 0, wasm: 0, css: 0 };
for (const file of readdirSync(assets)) {
  const kind = file.endsWith(".wasm")
    ? "wasm"
    : file.endsWith(".js")
      ? "js"
      : file.endsWith(".css")
        ? "css"
        : null;
  if (kind) totals[kind] += gzipSync(readFileSync(join(assets, file))).length / 1024;
}
let failed = false;
for (const [kind, size] of Object.entries(totals)) {
  const ok = size <= budgetsKiB[kind];
  failed ||= !ok;
  console.log(
    `${ok ? "ok  " : "FAIL"} ${kind.padEnd(4)} ${size.toFixed(1)} KiB gzip (budget ${budgetsKiB[kind]} KiB)`,
  );
}
if (failed) process.exit(1);
