// Writes the Android app's QR code (see android-app-qr.mjs).
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";

import { QR_PATH, androidAppQrSvg, root } from "./android-app-qr.mjs";

writeFileSync(resolve(root, QR_PATH), await androidAppQrSvg());
