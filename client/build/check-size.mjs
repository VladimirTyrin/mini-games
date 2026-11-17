// Fails the build when the gzip size of what the app can load exceeds the budget. Counts only
// assets reachable from index.html: Rollup may still emit chunks that nothing imports (e.g.
// Capacitor's web fallbacks in the web build), and those are never downloaded.
import { gzipSync } from "node:zlib";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const dist = resolve(dirname(fileURLToPath(import.meta.url)), "../dist");
// js: 190. The largest share is the protobuf runtime with the message schemas (~70 KiB); the
// setup forms, Stack Attack's animation and the PWA registration took it past 135, the sixth
// game (Word Hunt: board, keyboard, stats) and the snake's body renderer past 145, reconnect and
// version handling, the rules checks and the frame queue past 150, push notifications, friends
// and game invitations (screens, schemas, the invitation banner) past 160, the seventh game
// (Checkers: board, settings, schemas) past 170.
const budgetsKiB = { js: 190, wasm: 250, css: 30 };
const totals = { js: 0, wasm: 0, css: 0 };
const assetName = /[\w.-]+\.(?:js|wasm|css)\b/g;

const seen = new Set();
const pending = [...readFileSync(join(dist, "index.html"), "utf-8").matchAll(assetName)].map((m) => m[0]);
while (pending.length > 0) {
  const name = pending.pop();
  const path = join(dist, "assets", name);
  if (seen.has(name) || !existsSync(path)) continue;
  seen.add(name);
  const bytes = readFileSync(path);
  const kind = name.endsWith(".wasm") ? "wasm" : name.endsWith(".js") ? "js" : "css";
  totals[kind] += gzipSync(bytes).length / 1024;
  if (kind !== "wasm") pending.push(...[...bytes.toString("utf-8").matchAll(assetName)].map((m) => m[0]));
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
