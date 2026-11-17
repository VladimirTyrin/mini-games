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

## Pause in online real-time games

**Now.** A local real-time match (against bots) can be paused; an online one pauses only when a
seated player disconnects (budget per seat, drop vote; see [protocol.md](protocol.md)).

**Wanted.** A player pausing an online real-time match on purpose.

**Open questions** (to agree before building):
- How many pauses a player gets per match, and for how long each (or a shared time budget).
- Who may resume: only the player who paused, anyone after a while, or by vote.
- What happens when the pause runs out: resume with a countdown, or drop the player.
- Whether spectators see it and whether it counts against the disconnect budget.
