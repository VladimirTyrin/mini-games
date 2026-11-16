// Builds the Android app: the web bundle in its Android flavour, `cap sync`, then Gradle.
//   node build/android.mjs sync              web bundle + cap sync only
//   node build/android.mjs apk [debug|release]
// Signing comes from ../.keys/<variant>-signing.properties locally or from MG_ANDROID_* env vars
// in CI (see android/app/build.gradle).
import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const [command = "apk", variant = "release"] = process.argv.slice(2);
if (!["sync", "apk"].includes(command) || !["debug", "release"].includes(variant)) {
  console.error("usage: node build/android.mjs sync | apk [debug|release]");
  process.exit(2);
}

function run(program, args, options = {}) {
  const result = spawnSync(program, args, { stdio: "inherit", ...options });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

// Node CLIs run through this node binary: no shell, so nothing depends on PATH or .cmd shims.
function node(script, args, options) {
  run(process.execPath, [resolve(root, script), ...args], options);
}

const env = {
  ...process.env,
  VITE_PLATFORM: "android",
  VITE_SERVER_URL: process.env.MG_ANDROID_SERVER_URL ?? "ws://146.103.106.44/ws",
};
node("build/generate-proto.mjs", [], { cwd: root });
node("build/build-wasm.mjs", [], { cwd: root });
node("node_modules/vite/bin/vite.js", ["build"], { cwd: root, env });
node("node_modules/@capacitor/cli/bin/capacitor", ["sync", "android"], { cwd: root });
if (command === "apk") {
  const android = resolve(root, "android");
  const task = variant === "release" ? "assembleRelease" : "assembleDebug";
  if (process.platform === "win32") {
    run("cmd.exe", ["/d", "/c", resolve(android, "gradlew.bat"), task, "--no-daemon"], { cwd: android });
  } else {
    run("./gradlew", [task, "--no-daemon"], { cwd: android });
  }
}
