// Fingerprint of the native Android shell: everything that ends up in the APK except the web
// bundle. A web bundle may be installed over the air only into a shell with the same fingerprint
// (see docs/android.md, "Updates"); any change here makes the app offer a new APK instead.
import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

// Build outputs, IDE state and files `cap sync` regenerates. The regenerated Gradle files are a
// function of the plugin set, which the package versions below already cover; hashing them would
// see their state before this build's sync and change the fingerprint one build late.
const skippedDirectoryNames = new Set([
  ".gradle",
  "build",
  ".idea",
  ".cxx",
  ".externalNativeBuild",
  "captures",
]);
const skippedPaths = new Set([
  "dist",
  "local.properties",
  "capacitor-cordova-android-plugins",
  "capacitor.settings.gradle",
  "app/capacitor.build.gradle",
  "app/src/main/assets/public",
  "app/src/main/assets/capacitor.config.json",
  "app/src/main/assets/capacitor.plugins.json",
  "app/src/main/res/xml/config.xml",
]);
const skippedExtensions = [".iml", ".log", ".hprof", ".apk", ".aab"];
const nativePackage = /^node_modules\/(@capacitor|@capgo)\/[^/]+$/;

function isSkipped(relativePath, name, isDirectory) {
  if (skippedPaths.has(relativePath)) return true;
  if (isDirectory) return skippedDirectoryNames.has(name);
  return skippedExtensions.some((extension) => name.endsWith(extension));
}

/** Relative `/`-separated paths of the native project's source files, sorted. */
function nativeSourceFiles(androidRoot, prefix = "") {
  const files = [];
  const entries = readdirSync(join(androidRoot, prefix), { withFileTypes: true });
  for (const entry of entries) {
    const relativePath = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (isSkipped(relativePath, entry.name, entry.isDirectory())) continue;
    if (entry.isDirectory()) files.push(...nativeSourceFiles(androidRoot, relativePath));
    else if (entry.isFile()) files.push(relativePath);
  }
  return files.sort();
}

// A Windows checkout has CRLF text files where CI has LF; both must give the same fingerprint.
function normalizedContent(path) {
  const bytes = readFileSync(path);
  return bytes.includes(0) ? bytes : Buffer.from(bytes.toString("latin1").replaceAll("\r\n", "\n"), "latin1");
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

/** Installed versions of the packages that carry native code into the APK. */
function nativePackageVersions(packageLock) {
  const packages = JSON.parse(packageLock).packages ?? {};
  return Object.keys(packages)
    .filter((key) => nativePackage.test(key))
    .sort()
    .map((key) => `${key.slice("node_modules/".length)}@${packages[key].version}`);
}

/** 16 hex digits; `webClientRoot` holds `android/`, `capacitor.config.ts` and `package-lock.json`. */
export function nativeFingerprint(webClientRoot) {
  const androidRoot = join(webClientRoot, "android");
  const lines = [
    ...nativeSourceFiles(androidRoot).map(
      (path) => `android/${path} ${sha256(normalizedContent(join(androidRoot, path)))}`,
    ),
    `capacitor.config.ts ${sha256(normalizedContent(join(webClientRoot, "capacitor.config.ts")))}`,
    ...nativePackageVersions(readFileSync(join(webClientRoot, "package-lock.json"), "utf-8")),
  ];
  return sha256(lines.join("\n")).slice(0, 16);
}
