# Localization (English, Russian)

The app speaks English and Russian. The server does not: it never picks words for a player.
Everything a player reads is chosen by the client from its own dictionaries; what the server and
the Rust core send is codes and values, plus English text for logs, bots and old clients.

Status: design agreed, not built.

## Decisions

| Question | Decision |
|---|---|
| Languages | English and Russian. Adding a third is adding a dictionary, nothing else. |
| Choice | The device's language on first start (Russian → Russian, anything else → English); a Русский / English switch in Settings overrides it, remembered on the device. |
| Server | Not localized. Errors carry their code, refusals and settings problems an enum value (per game) and numbers; the client words them. |
| Push notifications | The server sends a text key and arguments. Android words them from the app's resources, Web Push in the service worker. |
| Game names | Translated (table below). The app's own name stays "Mini Games". |
| Russian tone | «ты», kept plain: "Твой ход", "Ты выиграл", but buttons as actions: "Войти через Google", "Играть снова". No exclamation-mark cheerfulness. |
| Privacy policy | Two pages, English and Russian; the app links to the one in its language. |
| Word Hunt | The word list follows the interface language until the player picks one themself. |
| Screen checks | `fit.mjs` finds clipped or overflowing text in both languages without screenshots: locally for the screens a change touches, in CI for all. Screenshots only to look at a layout. |

## Choosing the language

- `client/src/i18n/locale.ts`: `type Locale = "en" | "ru"`, the current locale as a ref.
- Start: the saved choice from `platform/storage.ts`, else `navigator.languages`: the first
  `ru*` → `ru`, the first other language → `en`.
- The Settings switch saves the choice and changes the ref; the screen redraws, nothing reloads.
- `<html lang>` follows the locale (hyphenation, screen readers).
- In the Android app the switch also sets the per-app locale (`AppCompatDelegate.setApplicationLocales`,
  through a small native method of our Capacitor shell). Android then words push notifications in
  the language of the switch, not only the phone's. Before Android 13 AppCompat keeps it itself
  (`autoStoreLocales`).
- The service worker cannot read `localStorage`, so the locale is also written to IndexedDB
  (`push-sw.js` reads it for Web Push).

## Client dictionaries

```
client/src/i18n/
  en.ts        the source: every text, nested by screen (home, lobby, match, settings, games.snake…)
  ru.ts        `export const ru: Messages = {…}`: the same shape, checked by the compiler
  messages.ts  `export type Messages = typeof en` (texts widened to string)
  plural.ts    Intl.PluralRules-based helper: one / few / many for Russian, one / other for English
  use-i18n.ts  `const { t, locale } = useI18n()`; `t` is the dictionary of the current locale
  format.ts    dates, times, numbers through Intl with the current locale
```

- No library: the dictionaries are plain typed objects. A missing Russian text is a compile
  error, a typo in a key is a compile error, and the bundle gains a few kilobytes of text only.
- Texts with values are functions: `wins: (count: number) => plural(count, {one: "победа",
  few: "победы", many: "побед"})`, `invites: (name: string) => \`${name} зовёт тебя играть\``.
  No string templates parsed at run time.
- Each game's plugin takes its texts from `t.games.<game>`: title, How to play, settings labels,
  bot levels, its board's messages.
- What players type (names, chat, lobby names) is shown as typed, never translated.
- Lint: `vue/no-bare-strings-in-template` (eslint-plugin-vue) fails on text written straight
  into a template, so new screens cannot skip the dictionary. Symbols and numbers are allowed.
- Replay file names use the game's title in the current language (they are only names).

## Text that comes from the server or the core

The rule: a string the server or the core produces is never shown to a player as is. It is
either a code or an enum value the client words, or a value (a name, a number) placed into the client's words.

### Error codes

`ErrorResponse.code` already says what went wrong. The client has a text per code
(`t.errors.lobbyFull`, …). `ErrorResponse.message` stays English, for logs, bots and the console;
the UI never shows it.

### Refusals of moves and settings problems

Today the core's reasons are English sentences (`CommandError::Illegal("this word was already
tried")`, `SettingProblem::RuleBroken { rule: "must be a power of two", .. }`), and Word Hunt's
board shows them. They become enums, from the game's rules to the client's dictionary:

- Each game crate has `enum Refusal` (why a move is refused: `AlreadyTried`, `NotInWordList`,
  `MustCapture`…) and, if its settings have rules a range cannot express, `enum SettingRule`
  (`PowerOfTwo`…). `Display` gives the English, for logs only.
- `Game` gets `type Refusal`; `CommandError` becomes generic over it: `GameOver`,
  `NotYourTurn`, `NotPlaying`, `Busy` stay common, `Illegal(Self::Refusal)` is the game's.
  `SettingProblem::RuleBroken { rule: SettingRule, value }` the same way; `OutOfRange` keeps
  its `min`/`max`/`value`, the client words it with the numbers.
- Wire: each game's proto file mirrors them (`enum WordHuntRefusal`, `enum
  Puzzle2048SettingRule`); `protocol.proto` gathers them:

  ```proto
  message Refusal {
    oneof reason {
      CommonRefusal common = 1;   // game over, not your turn, not playing, busy
      WordHuntRefusal word_hunt = 2;
      CheckersRefusal checkers = 3;
      // … one per game that refuses moves
    }
  }
  message ErrorResponse { ErrorCode code = 1; string message = 2; Refusal refusal = 3; }
  ```

  Settings problems get the same shape (`field` plus `OutOfRange` or the game's `SettingRule`),
  in `ErrorResponse` for the server and from WASM for forms and local play.
- `mg-registry` maps each Rust enum to its proto enum with an exhaustive `match`: a new variant
  without a wire value does not compile.
- The client words them with dictionaries typed `Record<WordHuntRefusal, string>`: a value
  without a text in either language does not compile either. No key lists, no cross-check tests.
- A value the client does not know (a newer server) falls back to the error code's text.

### Default lobby names

The server names a lobby "Vlad's Snake" when the host gives no name. It stops doing that: the
name stays empty, and the client shows `t.lobby.defaultName(host, game)`: "Vlad's Snake" in
English, "Змейка · Влад" in Russian (a name cannot be declined, so no "Лобби Влада").

### Bots

The client already names bots itself ("Careful bot 2" from the seat's bot kind and number);
those words move to the dictionary. The server's "Bot 3" in `display_name` stays for logs and
bot clients.

## Push notifications

The server keeps building notifications but sends keys instead of sentences. Its texts are an
enum too (`PushText::InviteTitle(GameKind)`, `InviteJoinedElsewhere`, …); the key is only how
FCM and the service worker name it (`push_invite_title_snake`), derived in one place.


- FCM: `android.notification.title_loc_key` / `body_loc_key` with `*_loc_args` (the inviter's
  name). Android looks the keys up in `client/android/app/src/main/res/values/strings.xml` and
  `values-ru/strings.xml`. The game's name cannot be an argument (Android puts arguments in
  verbatim), so the invitation has a key per game: `push_invite_title_snake`, …
- The top-level `notification.title/body` stay English: an app installed before this change
  has no such resources and shows them.
- Web Push: the payload carries `titleKey`, `bodyKey`, `args`; `push-sw.js` has the same keys
  for both languages (generated from the dictionaries at build time, so there is one source)
  and the locale from IndexedDB.
- A notification arriving while the app is open is worded by the app from the same keys.
- A test checks that every key the server can send exists in both `strings.xml` files and the
  service worker's table.
- The Android notification channel's name, created by the app, comes from the dictionary.

## Game names

To confirm:

| English | Russian |
|---|---|
| Snake | Змейка |
| Tic-Tac-Toe | Крестики-нолики |
| Stack Attack | Stack Attack (for now) |
| 2048 | 2048 |
| Numbers Match | Пары чисел |
| Word Hunt | Угадай слово |
| Checkers | Шашки |
| Sea Battle | Морской бой |

The core's `GameInfo::name` stays English (logs, bots, the arena); the client's titles come from
the dictionary.

## Privacy policy

`privacy.html` (English) and `privacy-ru.html`; Home and Settings link to the one of the current
locale, each page links to the other. AGENTS.md's privacy rule then covers both pages and both
"Last updated" dates.

## Checking that everything fits

Two tools over the same scenarios (`tools/screens/scenarios.mjs`: which route, what a player
clicks, for every screen, game and dialog):

- `tools/screens/fit.mjs` — the check, no pictures. Opens every scenario at every size of
  AGENTS.md "Screens" in both languages, several tabs at once, and reports, with the screen,
  size, language and the element's text:
  - the page scrolls sideways (`scrollWidth > innerWidth`);
  - an element's text is cut: `scrollWidth > clientWidth` or `scrollHeight > clientHeight` where
    it hides overflow or ellipsizes;
  - text spills out of its control (a button's or label's text box wider than the control).

  Exit code 1 when anything was found. `--games`, `--sizes`, `--langs` narrow it.
- `tools/screens/shoot.mjs` — the pictures, for looking at a layout, and for showing the user:
  every screen of the app, not only the games (home, local setup, online, lobby, friends,
  settings, saved replays, every game, How to play and the result over a game), or a subset
  (`--screens home,lobby,checkers`), at every size or a subset (`--sizes`), in every language
  or a subset (`--langs ru`). Files in `screens-out/<lang>/<screen>/<size>.png`; `all.html`
  shows them by screen with the languages side by side. Online screens (lobby, online) start
  the local server's lobby with bots, so they need the server running as well as `npm run dev`.
  Not a check: run when a layout needs a look, and send the sheet (`all.html` or the PNGs) to
  the user for review.

When they run:

- Locally, before committing a UI change: `fit.mjs` narrowed to the screens it touches — a
  quick answer.
- In CI, the whole of `fit.mjs` on every push that changes `client/` or `tools/screens/`: a job
  that builds the client, serves the build and runs it in the runner's Chrome. The safety net
  for what the local run left out.

Both set the locale before the app loads (the storage key the app reads). The scenarios stop
clicking buttons by their English text: buttons they press get `data-testid` (`play`,
`add-opponent`, `random`, `ready`), so one scenario drives both languages. Dialogs over a game
(How to play, the result) are scenarios too.

AGENTS.md "Screens" then reads: every size, in both languages, checked by `fit.mjs`.

## Rollout

On `feature/i18n`, merged when done:

1. `i18n/` with English only, every client text moved into it, the lint rule on. No visible change.
2. Core and server: refusal and setting-rule enums per game, their proto mirrors,
   `ErrorResponse.refusal`, empty default lobby names, keyed pushes.
3. Russian dictionary, the Settings switch, device-language default, Word Hunt default list.
4. Android: `strings.xml` in both languages, per-app locale; Web Push service worker.
5. Privacy policy in Russian.
6. `scenarios.mjs`, `fit.mjs` (local and the CI job), `shoot.mjs` in both languages; fix what
   `fit.mjs` finds.
7. Docs: this file, AGENTS.md (no text outside the dictionaries; screens in both languages;
   privacy in both), `docs/adding-a-game.md` (a game's texts and refusals),
   `docs/push-notifications.md`, `docs/protocol.md`.

Deploy with the full `./deploy.sh`: the server and the app's native shell both change.
