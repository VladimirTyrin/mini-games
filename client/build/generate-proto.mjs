// Generates TypeScript types for proto/ (src/proto is git-ignored).
import { execFileSync } from "node:child_process";
import { rmSync } from "node:fs";
import { delimiter, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
rmSync(resolve(root, "src/proto"), { recursive: true, force: true });
// Run buf's JS launcher with node directly: no shell, works the same on Windows and Linux.
execFileSync(process.execPath, [resolve(root, "node_modules/@bufbuild/buf/bin/buf"), "generate"], {
  cwd: root,
  stdio: "inherit",
  // buf finds protoc-gen-es on PATH.
  env: { ...process.env, PATH: `${resolve(root, "node_modules/.bin")}${delimiter}${process.env.PATH}` },
});
