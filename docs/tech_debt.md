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
