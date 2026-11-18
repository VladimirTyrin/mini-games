# Tech debt

Known gaps we chose not to close yet, with what closing them would take.

## Restart countdown for players

**Now.** Before a restart, `deploy.sh` drains the server: running matches play out, new ones are
refused (see [deploy.md](deploy.md), "Restarts and running matches"). Players get
`ServerUpdating` but no time: they do not know how long they have to finish.

**Wanted.** Something like "The server restarts in 2:45 — finish your game", counting down in the
match and the lobby.

**Sketch.**
- `deploy.sh` passes its limit to the drain endpoint, e.g. `POST /admin/drain?seconds=180`.
- The server keeps the deadline and sends the time left in `ServerUpdating`, like
  `expires_in_ms` on invitations, so device clocks do not matter.
- The app shows a banner that counts down. `ServerUpdating` arrives again on every later
  sign-in, so a reconnect picks up the same deadline.

## Network bots and private commands

**Now.** The bot SDK and the C ABI have no private commands. A network bot seated in Sea Battle
never places its fleet: the server places a random one when `secret_setup_ms` runs out, so the
match starts after 90 s.

**Wanted.** `SubmitSecretCommand` in the SDK, with a strategy hook for it (as `Bot::decide_secret`
for built-in bots), and the same in the C ABI.

## Placement countdown and device clocks

**Now.** The app counts the time to place a fleet down from `MatchHeader.started_at_unix_ms`, so
a device clock that is off shows the wrong time left (the server still decides).

**Wanted.** The time left from the server, as invitations do with `expires_in_ms`, also in a
catch-up.

## Pause in online real-time games

**Now.** A local real-time match (against bots) can be paused; an online one pauses only when a
seated player disconnects (a turn-based one only on that player's turn; see
[protocol.md](protocol.md)).

**Wanted.** A player pausing an online real-time match on purpose.

**Open questions** (to agree before building):
- How many pauses a player gets per match, and for how long each (or a shared time budget).
- Who may resume: only the player who paused, anyone after a while, or by vote.
- What happens when the pause runs out: resume with a countdown, or drop the player.
- Whether spectators see it and whether it counts against the disconnect budget.
