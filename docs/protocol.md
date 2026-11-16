# Protocol v2

Source of truth: `proto/minigames/v2/*.proto` (`buf lint` clean). This page explains the flow.

## Transport

- WebSocket at `/ws`, text frames, each one a `ClientMessage` / `ServerMessage` in the
  canonical protobuf JSON mapping (field names in lowerCamelCase, enums as names, 64-bit
  integers as strings).
- Replay files (`.mgreplay`) are binary protobuf `ReplayFile`.
- Limits:
  - 64 KiB per message. Only a signed-in connection may send a bigger frame, it must be a
    `ShareReplay` (up to 4 MiB of replay), and at most one such frame per 5 s
    (`RATE_LIMITED`). A bigger frame before sign-in is refused unread and the connection is
    closed.
  - Messages are rate-limited per connection (burst 60, then 40/s; chat 5, then 1/s).
  - Per session: `RequestReplay` once per 2 s, `RequestCatchUp` / `ReportDesync` (shared) once
    per second; earlier requests get `RATE_LIMITED`, retry later.
  - Connections per client IP are capped (default 32, HTTP 429 on upgrade).
  - A client that does not read its messages fast enough (queue over 512 messages or 8 MiB) is
    disconnected; it reconnects and resumes.

## Requests and pushes

`ClientMessage.request_id` (non-zero) is echoed in the reply; a failure is an
`ErrorResponse { code, message }` with the same id. Messages with `request_id = 0` from the
server are pushes (lobby updates, frames, chat, some errors).

## Session

1. Optional `ServerInfoRequest` → `ServerInfo` (protocol range, guests allowed, Google client
   id) — works before signing in.
2. `Hello { protocol_version: 2, credentials, resume_token }` → `Welcome { identity,
   session_token, resume_token, server_info, resumed }`, or `ERROR_CODE_PROTOCOL_UNSUPPORTED`
   (the client must not retry; the app needs an update). A connection that has not signed in
   within 10 s is closed, so connect when about to send `Hello`.
3. Credentials: guest name, Google ID token, bot key, or a `session_token` from an earlier
   `Welcome` (Google/bot sessions; HMAC-signed, 30 days; a bot token stops working once its key
   is removed from the server).
4. A player is identified by `Identity.id`; `display_name` is only a label (sanitized, at most 24
   characters) and several players may share it. Everything that refers to a player carries
   its `Identity` (seats, lobby host, lobby summaries, observers, play-again votes, chat
   senders, match and replay headers) or its id (`KickMember.player_id`); results, pauses and
   drop votes refer to seats by slot. Clients compare players by id, never by name, and
   disambiguate namesakes on screen themselves. Ids are opaque, with a prefix per kind:
   - `g_….…` Google account on one device: an HMAC of the Google subject keyed by the server's
     session secret (so the Google id itself is never shown to other players), then the session.
     Every device or browser tab of an account is its own player with its own id: an account
     can play from a phone and a laptop at once, even at one table;
   - `b_…` network bot: derived the same way from its configured name;
   - `u_…` guest: new for every session;
   - `bot-N` built-in bot: unique within its lobby.

   `Identity.avatar_url` is the Google profile picture (`https://`, hosted on
   `googleusercontent.com`, at most 512 bytes; anything else is dropped). It is empty for
   guests, bots and Google accounts without an acceptable picture. Session tokens carry it, so
   a session sign-in restores it; tokens issued before avatars existed are refused and the
   client signs in with Google again.
5. `resume_token` reclaims the same session after a dropped connection (grace period 60 s in
   lobbies; during a match the seat's pause budget applies).
6. A session is taken over only by a sign-in that resumes it (its `resume_token`, e.g. a
   duplicated browser tab) or, for network bots, by another sign-in with the same key (a
   restarted bot gets its seat back). The old connection receives
   `ERROR_CODE_SIGNED_IN_ELSEWHERE` (push) and is closed; a client must not reconnect
   automatically after it, or the two would keep taking the session from each other. A Google
   account signing in on another device gets a new session instead. A resumed account session
   whose name or avatar changed meanwhile takes the new ones, and its lobby receives an updated
   `LobbyState`. The web client keeps the resume token per tab; the Android app keeps it across
   restarts, so a killed app gets its seat back.
7. Ping every 15 s; the server closes connections silent for 45 s.
8. On shutdown the server pushes `ServerShuttingDown` and closes connections.

## Lobbies

`LobbyState` is always a full snapshot. Seat order is the in-game slot order; the host holds
seat 0 whenever a match starts. Phases: `OPEN → IN_MATCH → POST_MATCH (play-again votes) /
REPLAY (shared replay)`.

- The join reply (`LobbyState`) comes before anything about a running match or replay.
- `LobbyList` pushes to clients outside lobbies are coalesced (up to 250 ms) and sent only
  when a summary changed (seats, observers, phase, settings, name, host).
- `POST_MATCH` carries `last_match_id`, `last_result` and `play_again_votes`. When every seated
  human voted the next match starts; if it cannot (e.g. too few players), members get an
  `ERROR_CODE_INVALID_SETTINGS` push and the votes are cleared. Any change to who sits or to
  the settings (a player leaves, steps aside to watch or joins, a bot is added, the host edits
  the settings) clears the votes too: the next match would not be the one voted for (a Word Hunt
  group left with one player would even be a game alone), so everyone votes again.
- Lobbies outside a match close after an hour without activity (`LOBBY_CLOSED_REASON_INACTIVE`);
  chat counts as activity.

## Matches

```
server → MatchStarted(MatchHeader)          header: settings, seats (slot, identity, bot kind), tick_ms
server → Frame(TickFrame) …                 every tick / move; inputs + optional state_hash
client → SubmitCommand(match_id, GameCommand) → Ack | Error(COMMAND_REJECTED)
server → Paused(slot, budget_left_ms, drop_vote_in_ms) / Resumed(countdown_ms)
client → VoteDropPlayer(slot)               allowed once drop_vote_in_ms elapsed
client → ReportDesync / RequestCatchUp      → CatchUp(header, compact frames, next_tick, pauses,
                                               resume_countdown_ms)
server → Ended(MatchResult, final_tick)
client → RequestReplay(match_id)            → ReplayData(bytes)
```

- A seated player whose connection is gone when the match starts pauses it right away.
- A seat that was dropped (left, voted out, budget spent) is out for good: it may still watch,
  its commands get `COMMAND_REJECTED`, and its disconnects no longer pause the match.
- `Paused.pauses` lists every seat the match waits for and replaces the previous list (when one
  of two absent seats returns, the others are re-announced). `Resumed` clears it.
- `CatchUp.pauses` lists every seat the match waits for (`pause` repeats the first one for
  older clients); `resume_countdown_ms` is non-zero while the resume countdown runs.
- `Ended` without `result` means the server aborted the match (internal error): there is no
  replay and the lobby returns to `OPEN`.
- Submits, catch-ups and drop votes may be answered `RATE_LIMITED` when the match is too busy
  to queue them.

`FrameInput` is one of: a seat's `GameCommand`, a `GameWorldEvent` (randomness resolved by
the authority) or `dropped_slot`. The seed never leaves the authority.

## Shared replays

The host sends `ShareReplay`; the server checks the file (off its main task, one upload per
session at a time) and answers `Ack` or `ERROR_CODE_INVALID_REPLAY`. Members then receive
`SharedReplay`, and the lobby is in `REPLAY`. `ReplayControl` (host only when
`host_only_control`) is relayed to every member; `SET_SPEED` must be 25..=400 percent, `SEEK`
within the replay, `UNSPECIFIED` is refused. Members who join or resume during `REPLAY` get
`SharedReplay`, then the last `SET_SPEED` and the last `PLAY` / `PAUSE` / `SEEK`. `STOP` returns
the lobby to `OPEN`.

## Compatibility

`protocol_version` changes only on incompatible changes. Adding optional fields or messages
keeps the version.
