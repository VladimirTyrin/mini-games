# Localization (English, Russian)

The app speaks English and Russian. The server does not: it never picks words for a player.
Everything a player reads is chosen by the client from its own dictionaries; what the server and
the Rust core send is codes and values, plus English text for logs, bots and old clients.

Status: built on `feature/i18n`.

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
| Word Hunt | A new player's word list is in the interface language (`GamePlugin.defaultSettings`); once Word Hunt settings are remembered (after a first match), they win, the word list with them. |
| Screen checks | `fit.mjs` finds clipped or overflowing text in both languages without screenshots: locally for the screens a change touches, in CI for all. Screenshots only to look at a layout. |

## Choosing the language

- `client/src/i18n/locale.ts`: `type Locale = "en" | "ru"`, the current locale as a ref.
- Start: the saved choice from `platform/storage.ts`, else `navigator.languages`: the first
  `ru*` → `ru`, the first other language → `en`.
- The Settings switch saves the choice and changes the ref; the screen redraws, nothing reloads.
- `<html lang>` follows the locale (hyphenation, screen readers).
- In the Android app the switch also sets the per-app locale (`AppCompatDelegate.setApplicationLocales`,
  through a small native method of our Capacitor shell, `AppLocalePlugin`), but only for a
  language the player picked: one taken from the phone is left to follow the phone (an empty
  per-app locale), or Android and the WebView would keep it after the phone changes language.
  From Android 13 push notifications are then worded in the language of the switch; before it
  AppCompat keeps the pick (`autoStoreLocales`) for the app's screens only, and notifications
  follow the phone's language.
- The service worker cannot read `localStorage`, so the locale is also written to IndexedDB
  (`push-sw.js` reads it for Web Push).

## Client dictionaries

```
client/src/i18n/
  en/<area>.ts      the source, one file per area (common, errors, refusals, app, home, settings,
                    friends, replays, online, lobby, setup, match) and en/games/<game>.ts
  ru/<area>.ts      the same shape, each typed `Messages["<area>"]`: checked by the compiler
  en/index.ts       gathers the areas into `en`; ru/index.ts into `ru: Messages`
  messages.ts       `export type Messages = typeof en`
  index.ts          `t`: reads the current language's dictionary (`t.lobby.start`); reactive
  locale.ts         the current language, the device default, `setLocale`, `onLocale`
  plural.ts         `pluralEn(n, one, other)`, `pluralRu(n, one, few, many)` (Intl.PluralRules)
  format.ts         dates, times, numbers through Intl in the current language
```

- Every language ships in the bundle, imported statically: nothing is fetched to switch or to
  start in a language, so the APK (and its over-the-air bundle, a zip of the same `dist/`) works
  on its own, offline. Word Hunt's word lists of both languages are in `dist/words/` too, and the
  push notification texts in the APK's resources. Keep it so: no lazy-loaded dictionaries.
- No library: the dictionaries are plain typed objects. A missing Russian text is a compile
  error, a typo in a key is a compile error, and the bundle gains a few kilobytes of text only.
- Texts with values are functions: `wins: (count: number) => \`${count} ${pluralRu(count, "победа",
  "победы", "побед")}\``, `invites: (name: string) => \`${name} зовёт тебя играть\``.
  No string templates parsed at run time.
- Each game's plugin takes its texts from `t.games.<game>`: title, settings labels, bot levels,
  its board's messages.
- How to play is the exception: one page per language, `<Game>Help.en.vue` and
  `<Game>Help.ru.vue`, each the whole text written out with its markup (headings, lists, bold).
  Rules prose cut into "before / bold word / after" dictionary pieces reads badly and translates
  worse: word order and punctuation differ between languages. `GamePlugin.Help` is
  `Record<Locale, Component>`, so a game without a page in some language does not compile. What
  must match the rest of the UI still comes from code: numbers from the game's constants, names of
  buttons and settings from the dictionary (Sea Battle's Random and Ready, Snake's Wrap).
- What players type (names, chat, lobby names) is shown as typed, never translated.
- Lint: `vue/no-bare-strings-in-template` (eslint-plugin-vue) fails on text written straight
  into a template, so new screens cannot skip the dictionary. Symbols and numbers are allowed.
  The help pages (`src/games/**/*Help.*.vue`) are exempt: each is the text of one language.
- Replay file names use the game's title in the current language (they are only names).

## Text that comes from the server or the core

The rule: a string the server or the core produces is never shown to a player as is. It is
either a code or an enum value the client words, or a value (a name, a number) placed into the client's words.

### Error codes

`ErrorResponse.code` already says what went wrong. The client has a text per code
(`t.errors.lobbyFull`, …). `ErrorResponse.message` stays English, for logs, bots and the console;
the UI shows it only for a code it does not know (below).

### Refusals of moves and settings problems

The core's reasons are enums, from the game's rules to the client's dictionary (done):

- No rejection carries a string. Each game crate has `enum Refusal` in `rules/refusal.rs`: the
  reasons a player can meet (`AlreadyTried`, `MustCapture`…), then, grouped under a comment,
  the input no honest client or authority sends (`SpawnCellOccupied`, `NoSuchSeat`…).
  `EnglishText::english` gives each one's English (logs, bot clients) and `Display` writes it.
- `Game` has `type Refusal`; `CommandError<R>` is generic over it: `GameOver`, `NotYourTurn`,
  `NotPlaying`, `Busy` are common, `Illegal(R)` is the game's, and `Malformed(ProtocolMisuse)`
  the core's own misuse (a private command to a game without them, a default one for a seat
  that owes none).
- Settings: `SettingProblem::RuleBroken { rule: SettingRule, value }`, where `SettingRule` is
  one core enum of generic kinds (`PowerOfTwo`, `TooLargeForBoard`, `FixedForDailyPuzzle`)
  rather than an enum per game: only two games have such rules, and a kind is worded once for
  every game. `OutOfRange` keeps its `min`/`max`/`value`, the client words it with the numbers.
- Wire: each game's proto file mirrors its refusals (`enum WordHuntRefusal`); `refusal.proto`
  gathers them, with the settings problem (imported by `protocol.proto`):

  ```proto
  message Refusal {
    oneof reason {
      CommonRefusal common = 1;   // game over, not your turn, not playing, busy, malformed, paused
      SnakeRefusal snake = 2;
      // … one per game that refuses moves
    }
  }
  message SettingProblem { string field = 1; oneof problem { OutOfRange out_of_range = 2; BrokenRule broken_rule = 3; } }
  message ErrorResponse { …; Refusal refusal = 3; SettingProblem setting_problem = 4; }
  ```

  `COMMON_REFUSAL_PAUSED` is the server's own (a command while the match is paused).
  `ErrorResponse` carries them for the server; WASM returns them encoded for forms and local
  play (see [protocol.md](protocol.md)).
- Each game's `Refusal` maps to the wire with an exhaustive `match` in `impl WireRefusal` next
  to the game's `WireGame` in `mg-protocol`, where every other per-game wire mapping lives: a
  player-facing variant to its proto value, malformed input to `COMMON_REFUSAL_MALFORMED` (so
  the client words one generic "not allowed"). A new variant does not compile until it is given
  one or the other. `ProtocolMisuse` is `MALFORMED` too; `SettingRule` maps in `mg-protocol`
  likewise. `mg-registry` erases the game type into `Rejection { refusal, explanation }`.
- For bots and logs, `mg_protocol::refusal_english(&proto::Refusal)` turns a wire refusal back
  into the same English as the Rust enums (`setting_rule_english` for a `SettingRule`); the C
  ABI exposes it as `mg_refusal_english`. The server's `ErrorResponse.message` for a rejected
  command is the same English, except for malformed input: there it is the detail the wire
  leaves out ("cell is outside the board"), for the bot or developer who sent it. The app never
  uses either: it words refusals itself.
- Word Hunt has no `NotInWordList`: the word list is the client's check, the rules only see
  letters.
- The client words them with dictionaries typed `Record<WordHuntRefusal, string>`: a value
  without a text in either language does not compile either. No key lists, no cross-check tests.
- A code or enum value the client cannot word (only a newer server can send one: the local
  game's WASM core is the client's own version) falls back to the core's English that came with
  it (`ErrorResponse.message`, the same text bots get), and with no English to the raw code
  ("Так нельзя (checkers 9)", "Что-то пошло не так (99)"), so the player can still report it.

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
FCM and the service worker name it (`push_invite_body_snake`), derived in one place.


- FCM: `android.notification.title_loc_key` / `body_loc_key` with `*_loc_args` (the inviter's
  name). Android looks the keys up in `client/android/app/src/main/res/values/push_strings.xml`
  and `values-ru/push_strings.xml`, generated from `client/push-texts.json`. The game's name
  cannot be an argument (Android puts arguments in verbatim), so the invitation's body has a key
  per game: `push_invite_body_snake`, …
- FCM gets no plain `title`/`body`: Android shows plain ones whenever they are present and then
  never looks at the keys. An app shell from before the keys has no such resources and shows its
  name without text until the APK is updated.
- Web Push: the payload carries `titleKey`, `titleArgs`, `bodyKey`, `bodyArgs`; `push-sw.js` has the same keys
  for both languages (`public/push-texts.js`, generated from `client/push-texts.json` like the
  Android resources, so there is one source) and the locale from IndexedDB.
- A notification arriving while the app is open shows none of its text: the app acts on its
  data (an invitation banner, the test's timings), worded from the dictionaries.
- A client test checks that the generated files match `push-texts.json`
  (`npm run generate:push-texts` writes them).
- A server test checks that every key it can send has both languages in `push-texts.json`.
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

  Exit code 1 when anything was found, or when a screen could not be opened or its scenario
  did not reach it (a scenario's `reached`). `--screens`, `--sizes`, `--langs` narrow it.
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

Both set the locale before the app loads (`mini-games.locale` in localStorage). The scenarios stop
clicking buttons by their English text: buttons they press get `data-testid` (`play`,
`add-opponent`, `fleet-random`, `fleet-ready`, `tictactoe-cell`, …), so one scenario drives both
languages. Dialogs over a game
(How to play, the result) are scenarios too.

AGENTS.md "Screens" then reads: every size, in both languages, checked by `fit.mjs`.

## Rollout

On `feature/i18n`, merged when done:

1. `i18n/` with English only, every client text moved into it, the lint rule on. No visible change.
2. Core and server: refusal and setting-rule enums per game, their proto mirrors,
   `ErrorResponse.refusal`, empty default lobby names, keyed pushes.
3. Russian dictionary, the Settings switch, device-language default, Word Hunt default list.
4. Android: `push_strings.xml` in both languages, per-app locale; Web Push service worker.
5. Privacy policy in Russian.
6. `scenarios.mjs`, `fit.mjs` (local and the CI job), `shoot.mjs` in both languages; fix what
   `fit.mjs` finds.
7. Docs: this file, AGENTS.md (no text outside the dictionaries; screens in both languages;
   privacy in both), `docs/adding-a-game.md` (a game's texts and refusals),
   `docs/push-notifications.md`, `docs/protocol.md`.

Deploy with the full `./deploy.sh`: the server and the app's native shell both change.
