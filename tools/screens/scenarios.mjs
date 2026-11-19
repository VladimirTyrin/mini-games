// Every screen of the app as a player reaches it: a route, then what they click (page JS, may
// await). Buttons are found by `data-testid`, never by their text, so one scenario drives every
// language. `inGame`: a game or a dialog over one (also checked on phones held sideways);
// `needsServer`: online screens, which need the server (the app's /ws) as well. `reached`: a page
// expression true once the scenario got where it meant to (a click on a button that is not there
// does nothing, and the check would then look at the wrong screen); the visit fails otherwise.

const click = (testId) => `document.querySelector('[data-testid="${testId}"]')?.click()`;
const wait = (ms) => `await new Promise((r) => setTimeout(r, ${ms}))`;
/** Waits until `selector` is on the page (up to `ms`), then goes on either way. */
const waitFor = (selector, ms = 5000) =>
  `for (let waited = 0; waited < ${ms} && !document.querySelector(${JSON.stringify(selector)}); waited += 100) ${wait(100)}`;
const play = `${click("play")}; ${wait(1500)};`;
const key = (name) => `window.dispatchEvent(new KeyboardEvent('keydown', { key: '${name}' })); ${wait(250)}`;
const openHelp = `${click("how-to-play")}; ${wait(600)};`;
const present = (selector) => `!!document.querySelector(${JSON.stringify(selector)})`;
const absent = (selector) => `!document.querySelector(${JSON.stringify(selector)})`;
const freeCell = '[data-testid="tictactoe-cell"]:not(:disabled)';

/** A guest's name typed as a player would (Vue listens to `input`), then Play as guest. */
const signInAsGuest = [
  waitFor('[data-testid="guest-name"]'),
  `{ const field = document.querySelector('[data-testid="guest-name"]'); if (field) { field.value = "Alexandra"; field.dispatchEvent(new Event("input")); } }`,
  wait(100),
  click("play-as-guest"),
];

/** What a player does to get into each game's match. */
const GAME_STARTS = {
  snake: [click("add-opponent"), click("add-opponent"), wait(200), play, wait(4500)],
  tictactoe: [play, `document.querySelector('${freeCell}')?.click()`, wait(1200)],
  stack_attack: [play, wait(4500)],
  puzzle2048: [play, ...["ArrowLeft", "ArrowUp", "ArrowRight", "ArrowDown"].map(key)],
  numbers_match: [play, wait(500)],
  // Words of the interface's language, which the game defaults to.
  word_hunt: [
    play,
    `for (const letter of localStorage.getItem("mini-games.locale") === "ru" ? "книга" : "crane") { [...document.querySelectorAll("button")].find((b) => b.textContent.trim().toLowerCase() === letter)?.click(); ${wait(120)}; }`,
    wait(1200),
  ],
  checkers: [play, wait(1500)],
  sea_battle: [
    play,
    click("fleet-random"),
    wait(300),
    click("fleet-ready"),
    wait(1500),
    `for (let i = 0; i < 12; i++) { const open = [...(document.querySelectorAll('[role=grid]')[0]?.querySelectorAll('button:not(:disabled)') ?? [])]; open[(i * 37) % Math.max(1, open.length)]?.click(); ${wait(500)}; }`,
  ],
};

const GAMES = Object.keys(GAME_STARTS);

export const SCENARIOS = [
  { id: "home", route: "/", steps: [wait(800)], reached: present("main") },
  { id: "settings", route: "/settings", steps: [wait(800)] },
  { id: "friends", route: "/friends", steps: [wait(800)] },
  {
    id: "sign-in",
    route: "/online",
    needsServer: true,
    steps: [waitFor('[data-testid="guest-name"]'), wait(300)],
    reached: present('[data-testid="guest-name"]'),
  },
  {
    id: "online",
    route: "/online",
    needsServer: true,
    steps: [...signInAsGuest, waitFor('[data-testid="create-lobby"]'), wait(500)],
    reached: present('[data-testid="create-lobby"]'),
  },
  {
    id: "lobby",
    route: "/online",
    needsServer: true,
    steps: [
      ...signInAsGuest,
      waitFor('[data-testid="create-lobby"]'),
      click("create-lobby"),
      waitFor('[data-testid="add-bot"]'),
      click("add-bot"),
      wait(400),
      click("add-bot"),
      wait(800),
    ],
    reached: present('[data-testid="add-bot"]'),
  },
  ...GAMES.map((game) => ({
    id: `setup-${game}`,
    route: `/play/${game}`,
    steps: [wait(800)],
    reached: present('[data-testid="play"]'),
  })),
  ...GAMES.map((game) => ({
    id: game,
    route: `/play/${game}`,
    inGame: true,
    steps: GAME_STARTS[game],
    reached: absent('[data-testid="play"]'),
  })),
  {
    id: "sea_battle-placing",
    route: "/play/sea_battle",
    inGame: true,
    steps: [play, click("fleet-random"), wait(400)],
    reached: present('[data-testid="fleet-ready"]'),
  },
  ...GAMES.map((game) => ({
    id: `help-${game}`,
    route: `/play/${game}`,
    inGame: true,
    steps: [...GAME_STARTS[game], openHelp],
    reached: present("[role=dialog]"),
  })),
  {
    // The result over a game: a TicTacToe against a bot played out by taking the first free cell.
    id: "result",
    route: "/play/tictactoe",
    inGame: true,
    steps: [
      play,
      `for (let move = 0; move < 30 && !document.querySelector('[data-testid="match-result"]'); move++) { document.querySelector('${freeCell}')?.click(); ${wait(700)}; }`,
      wait(1500),
    ],
    reached: present('[data-testid="match-result"]'),
  },
];
