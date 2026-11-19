#!/usr/bin/env node
// Checks that every text fits, on every screen of the app, at every size of AGENTS.md "Screens",
// in every language: no text clipped, ellipsized or spilling out of its control, nothing
// scrolling sideways. No pictures (shoot.mjs takes those); exit code 1 when anything is found.
//
//   node tools/screens/fit.mjs [--url http://localhost:5174] [--screens home,lobby,checkers]
//                              [--sizes iphone-se,ipad] [--langs ru] [--parallel 4] [--require-server]
//
// The app must be served at --url (`npm run dev` in client/, or the server with --static-dir).
// Online screens (online, lobby) need the server too; without it they are skipped, or, with
// --require-server (CI), the check fails.

import { inParallel, launchChrome, runOptions, visit, visits, withRetries } from "./chrome.mjs";
import { FIT_CHECK } from "./page-checks.mjs";
import { serverIsUp } from "./server-probe.mjs";

const options = runOptions();
const online = await serverIsUp(options.url);
const all = visits(options);
const planned = all.filter(({ scenario }) => online || !scenario.needsServer);
if (planned.length < all.length) {
  if (process.argv.includes("--require-server")) {
    console.error(`The server does not answer at ${options.url}/ws: the online screens cannot be checked.`);
    process.exit(1);
  }
  console.warn(`The server does not answer at ${options.url}/ws: skipping the online screens.`);
}

const chrome = launchChrome(options);
let failedVisits = 0;
try {
  const findings = await inParallel(planned, options.parallel, async (planned) => {
    const name = `${planned.scenario.id} · ${planned.size.id} · ${planned.lang}`;
    const problems = await withRetries(name, () =>
      visit(chrome, options.url, planned, async (tab) => {
        const { result } = await tab.send("Runtime.evaluate", { expression: FIT_CHECK, returnByValue: true });
        return result.value;
      }),
    );
    if (problems === null) failedVisits++;
    return { name, problems: problems ?? [] };
  });
  let count = 0;
  for (const { name, problems } of findings) {
    for (const problem of problems) {
      count++;
      console.log(`${name}: ${problem.kind}: ${problem.element} "${problem.text}"`);
    }
  }
  console.log(`${planned.length} screens checked, ${count} problems, ${failedVisits} screens could not be opened.`);
  process.exitCode = count > 0 || failedVisits > 0 ? 1 : 0;
} finally {
  chrome.stop();
}
