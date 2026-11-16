// @vitest-environment node
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { nativeFingerprint } from "./native-fingerprint.mjs";

let root;

function write(path, content) {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), content);
}

function lockWith(versions) {
  const packages = { "": { name: "web-client" } };
  for (const [name, version] of Object.entries(versions)) packages[`node_modules/${name}`] = { version };
  return JSON.stringify({ lockfileVersion: 3, packages });
}

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "native-fingerprint-"));
  write("capacitor.config.ts", "export default { appId: 'app' };\n");
  write("android/app/build.gradle", "apply plugin: 'com.android.application'\n");
  write("android/app/src/main/AndroidManifest.xml", "<manifest/>\n");
  write("android/gradle/wrapper/gradle-wrapper.jar", Buffer.from([0x50, 0x4b, 0x00, 0x0d, 0x0a]));
  write(
    "package-lock.json",
    lockWith({ "@capacitor/android": "8.6.2", "@capgo/capacitor-updater": "8.54.2", vue: "3.5.45" }),
  );
});

afterEach(() => rmSync(root, { recursive: true, force: true }));

describe("nativeFingerprint", () => {
  it("is the same for the same native project", () => {
    expect(nativeFingerprint(root)).toMatch(/^[0-9a-f]{16}$/);
    expect(nativeFingerprint(root)).toBe(nativeFingerprint(root));
  });

  it("ignores Windows line endings in text files", () => {
    const before = nativeFingerprint(root);
    write("android/app/build.gradle", "apply plugin: 'com.android.application'\r\n");
    expect(nativeFingerprint(root)).toBe(before);
  });

  it("ignores build outputs, the copied web bundle and files cap sync regenerates", () => {
    const before = nativeFingerprint(root);
    write("android/app/build/outputs/apk/release/app-release.apk", "apk");
    write("android/.gradle/8.0/state.bin", "state");
    write("android/app/src/main/assets/public/index.html", "<html>");
    write("android/app/src/main/assets/capacitor.config.json", "{}");
    write("android/capacitor.settings.gradle", "include ':capacitor-app'");
    write("android/app/capacitor.build.gradle", "dependencies {}");
    write("android/dist/web-7.zip", "zip");
    write("android/local.properties", "sdk.dir=C:/sdk");
    expect(nativeFingerprint(root)).toBe(before);
  });

  it("changes with native sources, binary files and the Capacitor config", () => {
    const fingerprints = new Set([nativeFingerprint(root)]);
    write("android/app/src/main/AndroidManifest.xml", "<manifest package='x'/>\n");
    fingerprints.add(nativeFingerprint(root));
    write("android/gradle/wrapper/gradle-wrapper.jar", Buffer.from([0x50, 0x4b, 0x00, 0x0a]));
    fingerprints.add(nativeFingerprint(root));
    write("android/app/src/main/res/drawable/icon.xml", "<vector/>\n");
    fingerprints.add(nativeFingerprint(root));
    write("capacitor.config.ts", "export default { appId: 'other' };\n");
    fingerprints.add(nativeFingerprint(root));
    expect(fingerprints.size).toBe(5);
  });

  it("changes with Capacitor and Capgo package versions only", () => {
    const before = nativeFingerprint(root);
    write(
      "package-lock.json",
      lockWith({ "@capacitor/android": "8.6.2", "@capgo/capacitor-updater": "8.54.2", vue: "3.6.0" }),
    );
    expect(nativeFingerprint(root)).toBe(before);
    write(
      "package-lock.json",
      lockWith({ "@capacitor/android": "8.6.3", "@capgo/capacitor-updater": "8.54.2", vue: "3.6.0" }),
    );
    expect(nativeFingerprint(root)).not.toBe(before);
  });
});
