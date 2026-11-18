import type { LobbySeat } from "@/proto/minigames/v2/lobby_pb";
import type { MatchHeader } from "@/proto/minigames/v2/match_pb";
import { sameIdentity } from "@/util/players";

/**
 * Slots of the people at the table who lost the connection while the match goes on without
 * waiting for them (a turn-based match waits only on their turn, and then shows it as a pause):
 * what the lobby knows of connections, minus the seats the match waits for or dropped.
 */
export function awaySlots(
  header: MatchHeader,
  lobbySeats: readonly LobbySeat[],
  waitedFor: ReadonlySet<number>,
  dropped: ReadonlySet<number>,
): number[] {
  return header.seats
    .filter((seat) => seat.player && !waitedFor.has(seat.slot) && !dropped.has(seat.slot))
    .filter((seat) =>
      lobbySeats.some(
        (lobbySeat) => lobbySeat.connectionLost && sameIdentity(lobbySeat.occupant, seat.player),
      ),
    )
    .map((seat) => seat.slot);
}
