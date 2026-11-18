#!/usr/bin/env node
// Screenshots of every game at every screen size of AGENTS.md "Screens", for a look at the
// layouts side by side. Drives the installed Chrome headless over the DevTools protocol (no
// packages needed): each shot gets a fresh tab with the size and touch of the device, starts a
// local match the way a player would, and saves a PNG. Then one contact sheet per game.
//
//   node tools/screens/shoot.mjs [--url http://localhost:5174] [--out screens-out]
//                                [--games snake,checkers] [--sizes iphone-se,ipad-landscape]
//
// The app must be served at --url (`npm run dev` in client/); local matches need no server.

import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const SIZES = [
  { id: "iphone-se-old", label: "iPhone SE 1st 320×568", width: 320, height: 568, scale: 2, touch: true },
  { id: "iphone-se", label: "iPhone SE 375×667", width: 375, height: 667, scale: 2, touch: true },
  { id: "pixel-10a", label: "Pixel 10a 412×915", width: 412, height: 915, scale: 2, touch: true },
  { id: "pixel-10a-landscape", label: "Pixel 10a sideways 915×412", width: 915, height: 412, scale: 2, touch: true },
  { id: "ipad", label: "iPad 2018 768×1024", width: 768, height: 1024, scale: 2, touch: true },
  { id: "ipad-landscape", label: "iPad 2018 sideways 1024×768", width: 1024, height: 768, scale: 2, touch: true },
  { id: "laptop-half", label: "2K 16\" half 853×1067", width: 853, height: 1067, scale: 1, touch: false },
  { id: "laptop", label: "2K 16\" laptop 1707×1067", width: 1707, height: 1067, scale: 1, touch: false },
  { id: "monitor-half", label: "4K 27\" half 1280×1440", width: 1280, height: 1440, scale: 1, touch: false },
  { id: "monitor", label: "4K 27\" 2560×1440", width: 2560, height: 1440, scale: 1, touch: false },
];

const click = (text) => `[...document.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(${JSON.stringify(text)}))?.click()`;
const wait = (ms) => `await new Promise((r) => setTimeout(r, ${ms}))`;
const play = `${click("Play")}; ${wait(1500)};`;

/** Per game: what a player does before the shot (page JS, may await). `shots` names extra views. */
const GAMES = [
  { id: "home", route: "/", steps: [wait(800)] },
  { id: "snake", route: "/play/snake", steps: [click("+ Add opponent"), click("+ Add opponent"), wait(200), play, wait(4500)] },
  { id: "tictactoe", route: "/play/tictactoe", steps: [play, `document.querySelector('main button[aria-label]')?.click()`, wait(1200)] },
  { id: "stack_attack", route: "/play/stack_attack", steps: [play, wait(4500)] },
  {
    id: "puzzle2048",
    route: "/play/puzzle2048",
    steps: [play, ...["ArrowLeft", "ArrowUp", "ArrowRight", "ArrowDown"].map((key) => `window.dispatchEvent(new KeyboardEvent('keydown', { key: '${key}' })); ${wait(250)}`)],
  },
  { id: "numbers_match", route: "/play/numbers_match", steps: [play, wait(500)] },
  {
    id: "word_hunt",
    route: "/play/word_hunt",
    steps: [click("English"), play, ...[..."crane"].map((letter) => `${click(letter)}; ${wait(120)}`), wait(1200)],
  },
  { id: "checkers", route: "/play/checkers", steps: [play, wait(1500)] },
  { id: "sea_battle-placing", route: "/play/sea_battle", steps: [play, click("Random"), wait(400)] },
  {
    id: "sea_battle",
    route: "/play/sea_battle",
    steps: [
      play,
      click("Random"),
      wait(300),
      click("Ready"),
      wait(1500),
      `for (let i = 0; i < 12; i++) { const open = [...(document.querySelectorAll('[role=grid]')[0]?.querySelectorAll('button:not(:disabled)') ?? [])]; open[(i * 37) % Math.max(1, open.length)]?.click(); ${wait(500)}; }`,
    ],
  },
];

function option(name, fallback) {
  const at = process.argv.indexOf(`--${name}`);
  return at > 0 ? process.argv[at + 1] : fallback;
}

const url = option("url", "http://localhost:5174");
const out = resolve(option("out", "screens-out"));
const onlyGames = option("games", null)?.split(",");
const onlySizes = option("sizes", null)?.split(",");
const chromePath = option("chrome", "C:/Program Files/Google/Chrome/Application/chrome.exe");
const port = 9333;

mkdirSync(out, { recursive: true });

const chrome = spawn(
  chromePath,
  [
    "--headless=new",
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${mkdtempSync(join(tmpdir(), "mg-screens-"))}`,
    "--hide-scrollbars",
    "--no-first-run",
    "--no-default-browser-check",
    "about:blank",
  ],
  { stdio: "ignore" },
);

async function devtools(path, method = "GET") {
  for (let attempt = 0; attempt < 50; attempt++) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}${path}`, { method });
      const text = await response.text();
      // /json/close answers in plain text.
      return text.startsWith("{") ? JSON.parse(text) : text;
    } catch {
      await new Promise((r) => setTimeout(r, 200));
    }
  }
  throw new Error(`Chrome DevTools did not answer on ${port}`);
}

/** A fresh tab and a `send(method, params)` over its DevTools socket. */
async function openTab() {
  const target = await devtools("/json/new?about:blank", "PUT");
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolveOpen, reject) => {
    socket.onopen = resolveOpen;
    socket.onerror = reject;
  });
  let nextId = 1;
  const pending = new Map();
  socket.onmessage = (message) => {
    const data = JSON.parse(message.data);
    if (data.id && pending.has(data.id)) {
      const { resolveCall, reject } = pending.get(data.id);
      pending.delete(data.id);
      if (data.error) reject(new Error(data.error.message));
      else resolveCall(data.result);
    }
  };
  const send = (method, params = {}) =>
    new Promise((resolveCall, reject) => {
      const id = nextId++;
      pending.set(id, { resolveCall, reject });
      socket.send(JSON.stringify({ id, method, params }));
    });
  const close = async () => {
    socket.close();
    await devtools(`/json/close/${target.id}`);
  };
  return { send, close };
}

async function shoot(game, size) {
  const tab = await openTab();
  try {
    await tab.send("Emulation.setDeviceMetricsOverride", {
      width: size.width,
      height: size.height,
      deviceScaleFactor: size.scale,
      mobile: size.touch,
    });
    await tab.send("Emulation.setTouchEmulationEnabled", size.touch ? { enabled: true, maxTouchPoints: 5 } : { enabled: false });
    await tab.send("Page.enable");
    await tab.send("Page.navigate", { url: `${url}/#${game.route}` });
    await new Promise((r) => setTimeout(r, 2500));
    const script = `(async () => { ${game.steps.join(";\n")} })()`;
    await tab.send("Runtime.evaluate", { expression: script, awaitPromise: true });
    const shot = await tab.send("Page.captureScreenshot", { format: "png" });
    const file = join(out, `${game.id}--${size.id}.png`);
    writeFileSync(file, Buffer.from(shot.data, "base64"));
    return file;
  } finally {
    await tab.close();
  }
}

/** One page per game: every size scaled to the same height, labelled, shot as one image. */
async function contactSheet(game, sizes) {
  const rowHeight = 520;
  const cells = sizes
    .map(
      (size) =>
        `<figure><img src="${game.id}--${size.id}.png" style="height:${rowHeight}px"><figcaption>${size.label}</figcaption></figure>`,
    )
    .join("");
  const html = `<!doctype html><meta charset="utf-8"><style>
    body { margin: 0; padding: 16px; background: #0b0f17; color: #e5e7eb; font: 16px system-ui; }
    h1 { margin: 0 0 12px; font-size: 22px; }
    main { display: flex; flex-wrap: wrap; gap: 16px; align-items: flex-start; }
    figure { margin: 0; } img { display: block; border: 1px solid #374151; }
    figcaption { margin-top: 4px; color: #9ca3af; }
  </style><h1>${game.id}</h1><main>${cells}</main>`;
  const page = join(out, `${game.id}.html`);
  writeFileSync(page, html);
  const tab = await openTab();
  try {
    await tab.send("Emulation.setDeviceMetricsOverride", { width: 3000, height: 800, deviceScaleFactor: 1, mobile: false });
    await tab.send("Page.enable");
    await tab.send("Page.navigate", { url: pathToFileURL(page).href });
    await new Promise((r) => setTimeout(r, 1500));
    const { result } = await tab.send("Runtime.evaluate", { expression: "document.documentElement.scrollHeight" });
    const shot = await tab.send("Page.captureScreenshot", {
      format: "jpeg",
      quality: 85,
      captureBeyondViewport: true,
      clip: { x: 0, y: 0, width: 3000, height: result.value, scale: 0.6 },
    });
    const file = join(out, `sheet--${game.id}.jpg`);
    writeFileSync(file, Buffer.from(shot.data, "base64"));
    return file;
  } finally {
    await tab.close();
  }
}

try {
  const games = GAMES.filter((game) => !onlyGames || onlyGames.includes(game.id));
  const sizes = SIZES.filter((size) => !onlySizes || onlySizes.includes(size.id));
  for (const game of games) {
    for (const size of sizes) {
      const file = await shoot(game, size);
      console.log(file);
    }
    console.log(await contactSheet(game, sizes));
  }
} finally {
  chrome.kill();
}
