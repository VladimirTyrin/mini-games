import type { BotKind } from "@/proto/minigames/v2/game_pb";
import type { MatchHeader } from "@/proto/minigames/v2/match_pb";
import { seatLabels } from "@/util/players";

import { pluginBySettings } from ".";
import type { GamePlugin } from "./game-plugin";

/** A bot by its level and its number among the bots at the table: "Careful bot 2". */
export function botName(plugin: GamePlugin | undefined, kind: BotKind, number: number): string {
  return `${plugin?.botLabel(kind) ?? "Bot"} ${number}`;
}

/** Bots numbered from 1 in seat order, by slot index (`null` for a person's seat). */
export function botNumbers(seats: readonly { readonly botKind?: BotKind }[]): (number | null)[] {
  let bots = 0;
  return seats.map((seat) => (seat.botKind ? ++bots : null));
}

/** The names a match shows for its seats, by slot: people as they are called, bots by level. */
export function matchSeatNames(header: MatchHeader): string[] {
  const names = seatLabels(header);
  const plugin = pluginBySettings(header.settings?.game.case);
  const numbers = botNumbers(header.seats);
  header.seats.forEach((seat, index) => {
    const number = numbers[index];
    if (seat.botKind && number !== null && number !== undefined) {
      names[seat.slot] = botName(plugin, seat.botKind, number);
    }
  });
  return names;
}
