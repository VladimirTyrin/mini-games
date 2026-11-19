// Writes the push notification texts for Android and the web (see push-texts.mjs).
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

import { pushTextOutputs, root } from "./push-texts.mjs";

for (const [path, content] of Object.entries(pushTextOutputs())) {
  const target = resolve(root, path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, content);
}
