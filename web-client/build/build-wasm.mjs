// Builds crates/wasm into src/wasm/pkg (git-ignored): cargo → wasm-bindgen → wasm-opt.
// Needs: rustup target wasm32-unknown-unknown, wasm-bindgen-cli (same version as the crate),
// wasm-opt (binaryen).
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const webRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(webRoot, "..");
const targetDir = resolve(repoRoot, "target/web");
const outDir = resolve(webRoot, "src/wasm/pkg");
const run = (command, args) => execFileSync(command, args, { cwd: repoRoot, stdio: "inherit" });

run("cargo", [
  "build",
  "-p",
  "mg-wasm",
  "--target",
  "wasm32-unknown-unknown",
  "--profile",
  "wasm-release",
  "--target-dir",
  targetDir,
]);
mkdirSync(outDir, { recursive: true });
const wasm = resolve(targetDir, "wasm32-unknown-unknown/wasm-release/mg_wasm.wasm");
run("wasm-bindgen", [wasm, "--target", "web", "--out-dir", outDir, "--out-name", "mg_wasm"]);
const bindgenOut = resolve(outDir, "mg_wasm_bg.wasm");
if (process.env.MG_SKIP_WASM_OPT !== "1") {
  run("wasm-opt", [
    "-Oz",
    "--strip-debug",
    "--strip-producers",
    "--detect-features",
    bindgenOut,
    "-o",
    bindgenOut,
  ]);
}
if (existsSync(bindgenOut)) {
  console.log(`mg_wasm_bg.wasm: ${(statSync(bindgenOut).size / 1024).toFixed(0)} KiB`);
}
