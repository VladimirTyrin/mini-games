// Builds the Android app: the web bundle in its Android flavour, `cap sync`, then Gradle.
//   node build/android.mjs sync              web bundle + cap sync only
//   node build/android.mjs apk [debug|release]
// Signing comes from ../.keys/<variant>-signing.properties locally or from MG_ANDROID_* env vars
// in CI (see android/app/build.gradle). A release build also writes android/dist/mini-games.apk
// and android/dist/android.json (build number for the in-app update check; deploy.sh uploads
// both).
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
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

/** Commits on this branch: grows with every change, so later builds always install over earlier ones. */
function buildNumber() {
  if (process.env.MG_BUILD_NUMBER) return Number(process.env.MG_BUILD_NUMBER);
  const result = spawnSync("git", ["rev-list", "--count", "HEAD"], { cwd: root, encoding: "utf-8" });
  const count = Number(result.stdout?.trim());
  if (result.status !== 0 || !Number.isInteger(count)) {
    console.error("cannot count commits for the build number; set MG_BUILD_NUMBER");
    process.exit(1);
  }
  return count;
}

const build = buildNumber();
const env = {
  ...process.env,
  MG_BUILD_NUMBER: String(build),
  VITE_PLATFORM: "android",
  VITE_SERVER_URL: process.env.MG_ANDROID_SERVER_URL ?? "wss://braintvsminigames.xyz/ws",
};
node("build/generate-proto.mjs", [], { cwd: root });
node("build/build-wasm.mjs", [], { cwd: root });
node("node_modules/vite/bin/vite.js", ["build"], { cwd: root, env });
node("node_modules/@capacitor/cli/bin/capacitor", ["sync", "android"], { cwd: root });
if (command === "apk") {
  const android = resolve(root, "android");
  const task = variant === "release" ? "assembleRelease" : "assembleDebug";
  if (process.platform === "win32") {
    run("cmd.exe", ["/d", "/c", resolve(android, "gradlew.bat"), task, "--no-daemon"], { cwd: android, env });
  } else {
    run("./gradlew", [task, "--no-daemon"], { cwd: android, env });
  }
  if (variant === "release") {
    const cargo = readFileSync(resolve(root, "../Cargo.toml"), "utf-8");
    const version = /\[workspace\.package\][^[]*?version\s*=\s*"([^"]+)"/s.exec(cargo)?.[1] ?? "0.0.0";
    const dist = resolve(android, "dist");
    mkdirSync(dist, { recursive: true });
    copyFileSync(
      resolve(android, "app/build/outputs/apk/release/app-release.apk"),
      resolve(dist, "mini-games.apk"),
    );
    writeFileSync(
      resolve(dist, "android.json"),
      `${JSON.stringify({ build, versionName: `${version} (${build})`, apk: "mini-games.apk" }, null, 2)}
`,
    );
    console.log(`android/dist/mini-games.apk: ${version} build ${build}`);
  }
}
