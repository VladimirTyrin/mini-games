import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

import { QR_PATH, androidAppQrSvg, androidApkUrl, root } from "../../build/android-app-qr.mjs";

import { ANDROID_APK_URL } from "./android-app";

describe("the Android app's QR code", () => {
  it("encodes the download link (npm run generate:android-app-qr after changing it)", async () => {
    expect(androidApkUrl()).toBe(ANDROID_APK_URL);
    expect(readFileSync(resolve(root, QR_PATH), "utf8")).toBe(await androidAppQrSvg());
  });
});
