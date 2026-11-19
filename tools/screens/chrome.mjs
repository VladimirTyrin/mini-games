// The installed Chrome, headless, driven over the DevTools protocol (no packages needed), and
// the options fit.mjs and shoot.mjs share.

import { spawn } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { SCENARIOS } from "./scenarios.mjs";

/** AGENTS.md "Screens", in CSS pixels. */
export const SIZES = [
  { id: "iphone-se-old", label: "iPhone SE 1st 320×568", width: 320, height: 568, scale: 2, touch: true },
  { id: "iphone-se", label: "iPhone SE 375×667", width: 375, height: 667, scale: 2, touch: true },
  { id: "iphone-15-pro", label: "iPhone 15 Pro 393×852", width: 393, height: 852, scale: 3, touch: true },
  { id: "pixel-10a", label: "Pixel 10a 412×915", width: 412, height: 915, scale: 2, touch: true },
  { id: "pixel-10a-landscape", label: "Pixel 10a sideways 915×412", width: 915, height: 412, scale: 2, touch: true },
  { id: "ipad", label: "iPad 2018 768×1024", width: 768, height: 1024, scale: 2, touch: true },
  { id: "ipad-landscape", label: "iPad 2018 sideways 1024×768", width: 1024, height: 768, scale: 2, touch: true },
  { id: "laptop-half", label: "2K 16\" half 853×1067", width: 853, height: 1067, scale: 1, touch: false },
  { id: "laptop", label: "2K 16\" laptop 1707×1067", width: 1707, height: 1067, scale: 1, touch: false },
  { id: "monitor-half", label: "4K 27\" at 150% half 1280×1440", width: 1280, height: 1440, scale: 1, touch: false },
  { id: "monitor", label: "4K 27\" at 150% 2560×1440", width: 2560, height: 1440, scale: 1, touch: false },
];

export const LANGS = ["en", "ru"];

/** Phones held sideways: only a game itself (and dialogs over it) must work there. */
const SIDEWAYS = new Set(["pixel-10a-landscape"]);

export function option(name, fallback) {
  const at = process.argv.indexOf(`--${name}`);
  return at > 0 ? process.argv[at + 1] : fallback;
}

function listOption(name, all, describe) {
  const picked = option(name, null)?.split(",");
  if (!picked) return all;
  const unknown = picked.filter((id) => !all.some((item) => describe(item) === id));
  if (unknown.length > 0) {
    throw new Error(`unknown --${name}: ${unknown.join(", ")} (known: ${all.map(describe).join(", ")})`);
  }
  return all.filter((item) => picked.includes(describe(item)));
}

/** What to run: `--screens`, `--sizes`, `--langs` narrow it; `--url` is where the app is served. */
export function runOptions() {
  return {
    url: option("url", "http://localhost:5174"),
    screens: listOption("screens", SCENARIOS, (scenario) => scenario.id),
    sizes: listOption("sizes", SIZES, (size) => size.id),
    langs: listOption("langs", LANGS, (lang) => lang),
    chromePath: option("chrome", process.platform === "win32" ? "C:/Program Files/Google/Chrome/Application/chrome.exe" : "google-chrome"),
    port: Number(option("port", "9333")),
    parallel: Number(option("parallel", "4")),
  };
}

/** Every (screen, size, language) to visit; a screen other than a game skips phones held sideways. */
export function visits({ screens, sizes, langs }) {
  const all = [];
  for (const scenario of screens) {
    for (const size of sizes) {
      if (SIDEWAYS.has(size.id) && !scenario.inGame) continue;
      for (const lang of langs) all.push({ scenario, size, lang });
    }
  }
  return all;
}

export function launchChrome({ chromePath, port }) {
  const chrome = spawn(
    chromePath,
    [
      "--headless=new",
      `--remote-debugging-port=${port}`,
      `--user-data-dir=${mkdtempSync(join(tmpdir(), "mg-screens-"))}`,
      "--hide-scrollbars",
      "--no-first-run",
      "--no-default-browser-check",
      "--no-sandbox",
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

  /** A `send(method, params)` over a DevTools socket. */
  async function connect(socketUrl) {
    const socket = new WebSocket(socketUrl);
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
    return { send, close: () => socket.close() };
  }

  let browser = null;

  /**
   * A fresh tab in a browser context of its own: its own localStorage, so tabs in parallel
   * neither share a language nor remember each other's settings.
   */
  async function openTab() {
    browser ??= devtools("/json/version").then((version) => connect(version.webSocketDebuggerUrl));
    const control = await browser;
    const { browserContextId } = await control.send("Target.createBrowserContext");
    const { targetId } = await control.send("Target.createTarget", { url: "about:blank", browserContextId });
    const page = await connect(`ws://127.0.0.1:${port}/devtools/page/${targetId}`);
    const close = async () => {
      page.close();
      await control.send("Target.disposeBrowserContext", { browserContextId });
    };
    return { send: page.send, close };
  }

  return { openTab, stop: () => chrome.kill() };
}

/**
 * Opens a fresh tab at the visit's size and language, plays the scenario's steps and hands the
 * tab to `inspect` (a check or a screenshot). Each visit is its own tab: nothing carries over.
 */
/** A visit that takes longer is stuck (a page that reloaded under an awaited step): it fails and is retried. */
const VISIT_TIMEOUT_MS = 90_000;

export async function visit(chrome, url, planned, inspect) {
  let timer;
  const timeout = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`no answer in ${VISIT_TIMEOUT_MS / 1000} s`)), VISIT_TIMEOUT_MS);
  });
  try {
    return await Promise.race([visitOnce(chrome, url, planned, inspect), timeout]);
  } finally {
    clearTimeout(timer);
  }
}

async function visitOnce(chrome, url, { scenario, size, lang }, inspect) {
  const tab = await chrome.openTab();
  try {
    await tab.send("Emulation.setDeviceMetricsOverride", {
      width: size.width,
      height: size.height,
      deviceScaleFactor: size.scale,
      mobile: size.touch,
    });
    await tab.send("Emulation.setTouchEmulationEnabled", size.touch ? { enabled: true, maxTouchPoints: 5 } : { enabled: false });
    await tab.send("Page.enable");
    // Before the app reads them: the language, and nothing that pops up only the first time.
    await tab.send("Page.addScriptToEvaluateOnNewDocument", {
      source: `try {
        localStorage.setItem("mini-games.locale", ${JSON.stringify(lang)});
        localStorage.setItem("mini-games.get-the-app.dismissed", "1");
        localStorage.setItem("mini-games.help-seen", JSON.stringify(${JSON.stringify(HELP_GAMES)}));
      } catch {}`,
    });
    await tab.send("Page.navigate", { url: `${url}/#${scenario.route}` });
    await new Promise((r) => setTimeout(r, 2500));
    const script = `(async () => { ${scenario.steps.join(";\n")} })()`;
    const { exceptionDetails } = await tab.send("Runtime.evaluate", { expression: script, awaitPromise: true });
    if (exceptionDetails) throw new Error(exceptionDetails.exception?.description ?? exceptionDetails.text);
    if (scenario.reached) {
      const { result } = await tab.send("Runtime.evaluate", { expression: scenario.reached, returnByValue: true });
      if (result.value !== true) throw new Error(`did not reach the screen (${scenario.reached})`);
    }
    return await inspect(tab);
  } finally {
    await tab.close();
  }
}

const HELP_GAMES = ["snake", "tictactoe", "stack_attack", "puzzle2048", "numbers_match", "word_hunt", "checkers", "sea_battle"];

/** Runs `work` over `items`, `parallel` at a time, in order of the results. */
export async function inParallel(items, parallel, work) {
  const results = new Array(items.length);
  let next = 0;
  async function worker() {
    while (next < items.length) {
      const at = next++;
      results[at] = await work(items[at], at);
    }
  }
  await Promise.all(Array.from({ length: Math.max(1, parallel) }, worker));
  return results;
}

/** A visit that fails (a page reloading itself mid-script, say) gets two more tries. */
export async function withRetries(describe, attempt) {
  for (let tries = 1; ; tries++) {
    try {
      return await attempt();
    } catch (error) {
      console.error(`${describe}, attempt ${tries}: ${error.message}`);
      if (tries === 3) return null;
    }
  }
}
