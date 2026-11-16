# Protocol v2

Source of truth: `proto/minigames/v2/*.proto` (`buf lint` clean). This page explains the flow.

## Transport

- WebSocket at `/ws`, text frames, each one a `ClientMessage` / `ServerMessage` in the
  canonical protobuf JSON mapping (field names in lowerCamelCase, enums as names, 64-bit
  integers as strings).
- Replay files (`.mgreplay`) are binary protobuf `ReplayFile`.
- Limits: 64 KiB per message (4 MiB+ for `ShareReplay`), rate-limited per connection.

## Requests and pushes

`ClientMessage.request_id` (non-zero) is echoed in the reply; a failure is an
`ErrorResponse { code, message }` with the same id. Messages with `request_id = 0` from the
server are pushes (lobby updates, frames, chat).

## Session

1. Optional `ServerInfoRequest` → `ServerInfo` (protocol range, guests allowed, Google client
   id) — works before signing in.
2. `Hello { protocol_version: 2, credentials, resume_token }` → `Welcome { identity,
   session_token, resume_token, resumed }`, or `ERROR_CODE_PROTOCOL_UNSUPPORTED` (the client
   must not retry; the app needs an update).
3. Credentials: guest name, Google ID token, bot key, or a `session_token` from an earlier
   `Welcome` (Google/bot sessions; HMAC-signed, 30 days).
4. `resume_token` reclaims the same session after a dropped connection (grace period 60 s in
   lobbies; during a match the seat's pause budget applies).
5. Ping every 15 s; the server closes connections silent for 45 s.

## Lobbies

`LobbyState` is always a full snapshot. Seat order is the in-game slot order (host first).
Phases: `OPEN → IN_MATCH → POST_MATCH (play-again votes) / REPLAY (shared replay)`.

## Matches

```
server → MatchStarted(MatchHeader)          header: settings, seats (slot, identity, bot kind), tick_ms
server → Frame(TickFrame) …                 every tick / move; inputs + optional state_hash
client → SubmitCommand(match_id, GameCommand) → Ack | Error(COMMAND_REJECTED)
server → Paused(slot, budget_left_ms, drop_vote_in_ms) / Resumed(countdown_ms)
client → VoteDropPlayer(slot)               allowed once drop_vote_in_ms elapsed
client → ReportDesync / RequestCatchUp      → CatchUp(header, compact frames, next_tick, pause)
server → Ended(MatchResult, final_tick)
client → RequestReplay(match_id)            → ReplayData(bytes)
```

`FrameInput` is one of: a seat's `GameCommand`, a `GameWorldEvent` (randomness resolved by
the authority) or `dropped_slot`. The seed never leaves the authority.

## Compatibility

`protocol_version` changes only on incompatible changes. Adding optional fields or messages
keeps the version.
