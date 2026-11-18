// Lines of code by area: `node tools/loc/count.mjs` prints Markdown tables.
// Counts only files tracked by git. "Code" is a line that is neither blank nor only a comment;
// "tests" is the part of the code that lives in tests (test files, `tests/` modules and
// top-level `#[cfg(test)] mod` blocks). Docs, data (word lists, the neural model), lock files,
// JSON and images are not code and are left out.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

const GAMES = [
  ["snake", "Snake", "snake"],
  ["tictactoe", "TicTacToe", "tictactoe"],
  ["stack-attack", "Stack Attack", "stack_attack"],
  ["puzzle2048", "2048", "puzzle2048"],
  ["numbers-match", "Numbers Match", "numbers_match"],
  ["word-hunt", "Word Hunt", "word_hunt"],
  ["checkers", "Checkers", "checkers"],
  ["sea-battle", "Sea Battle", "sea_battle"],
];

const COMMENT_STYLES = {
  slash: { line: ["//"], block: ["/*", "*/"] },
  hash: { line: ["#"] },
  dash: { line: ["--"] },
  markup: { block: ["<!--", "-->"] },
  vue: { line: ["//"], block: ["/*", "*/"], extraBlock: ["<!--", "-->"] },
};

const LANGUAGE_BY_EXTENSION = {
  rs: "slash", ts: "slash", mjs: "slash", js: "slash", cs: "slash", java: "slash",
  gradle: "slash", proto: "slash", h: "slash", css: "slash", tmpl: "slash",
  vue: "vue", html: "markup", xml: "markup", axaml: "markup",
  py: "hash", sh: "hash", yml: "hash", yaml: "hash", pro: "hash", conf: "hash",
  service: "hash", template: "hash", sql: "dash",
};

const EXCLUDED = [
  /^client\/android\/gradlew(\.bat)?$/, // generated Gradle wrapper
  /^client\/public\/privacy\.html$/, // legal text, not code
];

const ANDROID_FILES = [
  /^client\/android\//,
  /^client\/capacitor\.config\.ts$/,
  /^client\/build\/(android|native-fingerprint)/,
  /^client\/src\/platform\/(android-app|native-app|app-update|web-bundle)\b/,
  /^client\/src\/stores\/app-update\b/,
];

/** Area of a file as [section, row]; null when the file is not counted. */
function classify(path) {
  let match = path.match(/^crates\/games\/([^/]+)\//);
  if (match) return ["core", `game:${match[1]}`];
  if (path.startsWith("crates/core/")) return ["core", "mg-core"];
  if (path.startsWith("crates/registry/")) return ["core", "registry"];

  if (path.startsWith("crates/server/")) return ["server", "server"];

  match =
    path.match(/^proto\/minigames\/v2\/([^/.]+)\.proto$/) ??
    path.match(/^crates\/protocol\/src\/games\/([a-z0-9_]+?)(_tests)?\.rs$/);
  if (match) {
    const game = GAMES.find(([, , snakeCase]) => snakeCase === match[1]);
    if (game) return ["protocol", `game:${game[0]}`];
  }
  if (path.startsWith("proto/")) return ["protocol", "proto-common"];
  if (path.startsWith("crates/protocol/")) return ["protocol", "protocol-crate"];

  if (path.startsWith("client/")) {
    if (ANDROID_FILES.some((pattern) => pattern.test(path))) return ["client", "android"];
    match = path.match(/^client\/src\/games\/([^/]+)\//);
    if (match) return ["client", `game:${match[1]}`];
    return ["client", "web"];
  }
  if (path.startsWith("crates/wasm/")) return ["client", "wasm"];

  if (path.startsWith("crates/bot-sdk/")) return ["bots", "bot-sdk"];
  if (path.startsWith("crates/arena/")) return ["bots", "arena"];
  if (path.startsWith("crates/ffi/")) return ["bots", "ffi"];
  if (path.startsWith("network-bots/")) return ["bots", "network-bots"];

  if (path.startsWith("xtask/")) return ["tooling", "xtask"];
  if (path.startsWith("tools/")) return ["tooling", "tools"];
  if (path.startsWith("deploy") || path.startsWith(".github/")) return ["tooling", "deploy-ci"];
  return null;
}

function isTestFile(path) {
  return (
    /\.test\.(ts|mjs|js)$/.test(path) ||
    /\/tests?\//.test(path) ||
    /(^|\/|_)tests?\.rs$/.test(path) ||
    /\/(androidTest|test)\/java\//.test(path)
  );
}

function countFile(path, language) {
  const style = COMMENT_STYLES[language];
  const blocks = [style.block, style.extraBlock].filter(Boolean);
  const wholeFileIsTest = isTestFile(path);
  const result = { code: 0, tests: 0, comments: 0, blank: 0 };
  let openBlockEnd = null;
  let inTestModule = false;
  let pendingCfgTest = false;

  for (const rawLine of readFileSync(join(root, path), "utf8").split(/\r?\n/)) {
    const line = rawLine.trim();
    if (line === "") {
      result.blank++;
      continue;
    }
    if (path.endsWith(".rs")) {
      if (rawLine.startsWith("#[cfg(test)]")) pendingCfgTest = true;
      else if (pendingCfgTest && /^(pub(\([^)]*\))? )?mod \w+ \{/.test(rawLine)) inTestModule = true;
      else if (!line.startsWith("#[")) pendingCfgTest = false;
    }
    if (isCommentLine(line, style, blocks, () => openBlockEnd, (end) => (openBlockEnd = end))) {
      result.comments++;
    } else {
      result.code++;
      if (wholeFileIsTest || inTestModule || pendingCfgTest) result.tests++;
    }
    if (inTestModule && rawLine === "}") inTestModule = false;
  }
  return result;
}

function isCommentLine(line, style, blocks, getOpenEnd, setOpenEnd) {
  const openEnd = getOpenEnd();
  if (openEnd) {
    if (line.includes(openEnd)) {
      setOpenEnd(null);
      return line.endsWith(openEnd);
    }
    return true;
  }
  if (style.line?.some((marker) => line.startsWith(marker))) return true;
  for (const [start, end] of blocks) {
    if (!line.startsWith(start)) continue;
    const closing = line.indexOf(end, start.length);
    if (closing === -1) {
      setOpenEnd(end);
      return true;
    }
    if (closing + end.length === line.length) return true;
  }
  return false;
}

const totals = new Map();
const add = (key, counts, files = 1) => {
  const entry = totals.get(key) ?? { files: 0, code: 0, tests: 0, comments: 0, blank: 0 };
  entry.files += files;
  for (const field of ["code", "tests", "comments", "blank"]) entry[field] += counts[field];
  totals.set(key, entry);
};

const tracked = execFileSync("git", ["ls-files", "-z"], { cwd: root, encoding: "utf8" })
  .split("\0")
  .filter(Boolean);
for (const path of tracked) {
  const extension = path.includes(".") ? path.slice(path.lastIndexOf(".") + 1) : "";
  const language = LANGUAGE_BY_EXTENSION[extension] ?? (path === "deploy.sh" ? "hash" : undefined);
  if (!language || EXCLUDED.some((pattern) => pattern.test(path))) continue;
  const area = classify(path);
  if (!area) continue;
  const counts = countFile(path, language);
  add(area.join("/"), counts);
  add(area[0], counts);
  add("all", counts);
  if (area[1].startsWith("game:")) add(`${area[0]}/games`, counts);
  add(`lang:${extension}`, counts);
}

const empty = { files: 0, code: 0, tests: 0, comments: 0, blank: 0 };
const get = (key) => totals.get(key) ?? empty;
const number = (value) => value.toLocaleString("en-US").replace(/,/g, " ");
const header = "| | Files | Code | of it tests | Comments | Blank |\n|---|--:|--:|--:|--:|--:|";
const row = (label, key, bold = false) => {
  const entry = get(key);
  const cells = [entry.files, entry.code, entry.tests, entry.comments, entry.blank].map(number);
  const wrap = (text) => (bold ? `**${text}**` : text);
  return `| ${wrap(label)} | ${cells.map(wrap).join(" | ")} |`;
};
const gameRows = (section) =>
  GAMES.map(([slug, name]) => row(`&nbsp;&nbsp;${name}`, `${section}/game:${slug}`));

const sections = [
  ["Core logic (Rust)", "core", [
    row("mg-core", "core/mg-core"),
    row("registry", "core/registry"),
    row("Games, all", "core/games"),
    ...gameRows("core"),
  ]],
  ["Server (Rust)", "server", [row("crates/server", "server/server")]],
  ["Protocol", "protocol", [
    row(".proto, common", "protocol/proto-common"),
    row("crates/protocol, common", "protocol/protocol-crate"),
    row("Games, all (.proto + Rust mapping)", "protocol/games"),
    ...gameRows("protocol"),
  ]],
  ["Client", "client", [
    row("Android-specific", "client/android"),
    row("Web, common", "client/web"),
    row("WASM bridge (crates/wasm)", "client/wasm"),
    row("Games, all", "client/games"),
    ...gameRows("client"),
  ]],
  ["Bots", "bots", [
    row("bot-sdk", "bots/bot-sdk"),
    row("arena", "bots/arena"),
    row("ffi (C ABI)", "bots/ffi"),
    row("network-bots (C#)", "bots/network-bots"),
  ]],
  ["Tooling and deploy", "tooling", [
    row("xtask", "tooling/xtask"),
    row("tools/", "tooling/tools"),
    row("deploy, CI", "tooling/deploy-ci"),
  ]],
];

const output = [];
for (const [title, key, rows] of sections) {
  output.push(`## ${title}`, "", header, ...rows, row("Total", key, true), "");
}

output.push("## Per game, everywhere", "", "| Game | Rules (Rust) | Client | Protocol | Total |", "|---|--:|--:|--:|--:|");
for (const [slug, name] of GAMES) {
  const parts = ["core", "client", "protocol"].map((section) => get(`${section}/game:${slug}`).code);
  output.push(`| ${name} | ${[...parts, parts.reduce((a, b) => a + b, 0)].map(number).join(" | ")} |`);
}
output.push("");

output.push("## Summary", "", header);
for (const [title, key] of sections) output.push(row(title, key));
output.push(row("Total", "all", true), "");

output.push("## By language", "", header);
const languages = [...totals.keys()]
  .filter((key) => key.startsWith("lang:"))
  .sort((a, b) => get(b).code - get(a).code);
for (const key of languages) output.push(row(key.slice(5), key));
output.push("");

console.log(output.join("\n"));
