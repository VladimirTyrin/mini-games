// The QR code of the Android app's download link, for a computer's screen: a phone's camera
// opens the link. The link is a constant (src/platform/android-app.ts), so the code is drawn once
// here, not in the page (no QR library in the bundle); a test checks the file still matches.
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import QRCode from "qrcode";

export const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const QR_PATH = "src/assets/android-app-qr.svg";

/** ANDROID_APK_URL as src/platform/android-app.ts declares it. */
export function androidApkUrl() {
  const source = readFileSync(resolve(root, "src/platform/android-app.ts"), "utf8");
  const match = /ANDROID_APK_URL = "([^"]+)"/.exec(source);
  if (!match) throw new Error("ANDROID_APK_URL not found in src/platform/android-app.ts");
  return match[1];
}

/** Dark modules on white with a quiet zone: what cameras read best, on any theme. */
export function androidAppQrSvg() {
  return QRCode.toString(androidApkUrl(), {
    type: "svg",
    errorCorrectionLevel: "M",
    margin: 2,
    color: { dark: "#000000", light: "#ffffff" },
  });
}
