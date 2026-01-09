import { execSync } from "child_process";
import { existsSync, mkdirSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const rootDir = resolve(__dirname, "..");
const outputDir = resolve(rootDir, "src/proto");

if (!existsSync(outputDir)) {
  mkdirSync(outputDir, { recursive: true });
}

console.log("Generating TypeScript from proto files...");
console.log(`Output directory: ${outputDir}`);

try {
  execSync("npx buf generate", { stdio: "inherit", cwd: rootDir });
  console.log("Proto generation complete!");
} catch (error) {
  console.error("Failed to generate proto files:", error.message);
  process.exit(1);
}
