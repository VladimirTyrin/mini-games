import type { Identity } from "@/proto/minigames/v2/identity_pb";
import type { MatchHeader } from "@/proto/minigames/v2/match_pb";

/** Players are told apart by id only: display names are labels and several players may share one. */
export function sameIdentity(a: Identity | undefined | null, b: Identity | undefined | null): boolean {
  return !!a && !!b && a.kind === b.kind && a.id === b.id;
}

const nameKey = (name: string) => name.trim().toLocaleLowerCase();

/**
 * Labels for players shown together (one lobby, match or chat), keyed by `Identity.id`. A name
 * shared by several of them gets a number from the second player on ("Alex", "Alex · 2"), in the
 * order given, so the screen stays unambiguous; the server never sees these labels.
 */
export function playerLabels(players: Iterable<Identity | undefined | null>): Map<string, string> {
  const labels = new Map<string, string>();
  const seenPerName = new Map<string, number>();
  for (const player of players) {
    if (!player || labels.has(player.id)) continue;
    const key = nameKey(player.displayName);
    const seen = (seenPerName.get(key) ?? 0) + 1;
    seenPerName.set(key, seen);
    labels.set(player.id, seen === 1 ? player.displayName : `${player.displayName} · ${seen}`);
  }
  return labels;
}

/** Label of every seat of a match, indexed by slot. */
export function seatLabels(header: MatchHeader): string[] {
  const labels = playerLabels(header.seats.map((seat) => seat.player));
  const bySlot: string[] = [];
  for (const seat of header.seats) {
    bySlot[seat.slot] = (seat.player && labels.get(seat.player.id)) || `Player ${seat.slot + 1}`;
  }
  return bySlot;
}
