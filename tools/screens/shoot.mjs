#!/usr/bin/env node
// Screenshots of the app's screens at the sizes of AGENTS.md "Screens", in its languages, to
// look at layouts side by side (fit.mjs is the check). Everything by default; narrow it with
//
//   node tools/screens/shoot.mjs [--url http://localhost:5174] [--out screens-out]
//                                [--screens home,lobby,checkers] [--sizes iphone-se,ipad]
//                                [--langs ru] [--parallel 4]
//
// Files: <out>/<lang>/<screen>/<size>.png; <out>/all.html shows every shot there (this run's
// and earlier ones'), by screen, the languages side by side. The app must be served at --url;
// online screens (online, lobby) need the server too, and are skipped without it.

import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import { LANGS, SIZES, inParallel, launchChrome, option, runOptions, visit, visits, withRetries } from "./chrome.mjs";
import { SCENARIOS } from "./scenarios.mjs";
import { serverIsUp } from "./server-probe.mjs";

const options = runOptions();
const out = resolve(option("out", "screens-out"));
const online = await serverIsUp(options.url);
const all = visits(options);
const planned = all.filter(({ scenario }) => online || !scenario.needsServer);
if (planned.length < all.length) {
  console.warn(`The server does not answer at ${options.url}/ws: skipping the online screens.`);
}

const shotPath = (lang, screen, size) => join(lang, screen, `${size}.png`);

const chrome = launchChrome(options);
try {
  await inParallel(planned, options.parallel, async (planned) => {
    const { scenario, size, lang } = planned;
    const file = join(out, shotPath(lang, scenario.id, size.id));
    const saved = await withRetries(`${scenario.id} · ${size.id} · ${lang}`, () =>
      visit(chrome, options.url, planned, async (tab) => {
        const shot = await tab.send("Page.captureScreenshot", { format: "png" });
        mkdirSync(dirname(file), { recursive: true });
        writeFileSync(file, Buffer.from(shot.data, "base64"));
        return file;
      }),
    );
    if (saved) console.log(saved);
  });
} finally {
  chrome.stop();
}

const STYLE = `<style>
  body { margin: 0; padding: 16px; background: #0b0f17; color: #e5e7eb; font: 15px system-ui; }
  nav { position: sticky; top: 0; background: #0b0f17; padding: 8px 0; z-index: 1; line-height: 1.9; }
  nav a { color: #93c5fd; margin-right: 10px; }
  h1 { margin: 28px 0 8px; font-size: 22px; }
  h2 { margin: 12px 0 6px; font-size: 15px; color: #9ca3af; font-weight: 600; }
  .row { display: flex; gap: 14px; align-items: flex-start; overflow-x: auto; padding-bottom: 6px; }
  figure { margin: 0; flex: none; } img { display: block; border: 1px solid #374151; height: 380px; }
  figcaption { margin-top: 4px; color: #9ca3af; font-size: 13px; }
</style>`;

/** all.html: every screen, one row of sizes per language, each shot opening at full size. */
function writeAllPage() {
  const sections = SCENARIOS.map((scenario) => {
    const rows = LANGS.map((lang) => {
      const cells = SIZES.filter((size) => existsSync(join(out, shotPath(lang, scenario.id, size.id))))
        .map((size) => {
          const src = shotPath(lang, scenario.id, size.id).replaceAll("\\", "/");
          return `<figure><a href="${src}"><img src="${src}" loading="lazy"></a><figcaption>${size.label}</figcaption></figure>`;
        })
        .join("");
      return cells ? `<h2>${lang.toUpperCase()}</h2><div class="row">${cells}</div>` : "";
    }).join("");
    return rows ? `<h1 id="${scenario.id}">${scenario.id}</h1>${rows}` : "";
  }).filter(Boolean);
  const links = SCENARIOS.filter((scenario) => sections.some((section) => section.includes(`id="${scenario.id}"`)))
    .map((scenario) => `<a href="#${scenario.id}">${scenario.id}</a>`)
    .join("");
  const page = join(out, "all.html");
  writeFileSync(page, `<!doctype html><meta charset="utf-8"><title>Screens</title>${STYLE}<nav>${links}</nav>${sections.join("")}`);
  return page;
}

console.log(writeAllPage());
