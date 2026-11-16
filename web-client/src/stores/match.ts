import { create, fromBinary } from "@bufbuild/protobuf";
import { defineStore } from "pinia";
import { shallowRef, watch } from "vue";

import type { BotKind, GameSettings } from "@/proto/minigames/v2/game_pb";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { type MatchHeader, MatchHeaderSchema } from "@/proto/minigames/v2/match_pb";
import { ReplayFileSchema } from "@/proto/minigames/v2/replay_pb";
import { LocalMatchSource } from "@/local/local-match-source";
import { MatchController } from "@/match/match-controller";
import type { MatchSource } from "@/match/match-source";
import { OnlineMatchSource } from "@/match/online-match-source";
import { ReplayMatchSource } from "@/match/replay-match-source";
import { downloadBytes } from "@/platform/download";

import { useConnectionStore } from "./connection";
import { sameIdentity } from "./lobby";
import { useToastStore } from "./toasts";

export interface LocalSeat {
  /** `null` for the local player. */
  bot: BotKind | null;
}

/** The match on screen (online, local or replay) and its controller. */
export const useMatchStore = defineStore("match", () => {
  const connection = useConnectionStore();
  const toasts = useToastStore();
  const controller = shallowRef<MatchController | null>(null);
  const replay = shallowRef<ReplayMatchSource | null>(null);

  function run(source: MatchSource): void {
    close();
    controller.value = new MatchController(source);
    source.start();
  }

  /** Offline match: you plus built-in bots, authority in a Web Worker. */
  function startLocal(settings: GameSettings, seats: LocalSeat[], playerName: string): void {
    const header = create(MatchHeaderSchema, {
      matchId: "local",
      settings,
      tickMs: 0,
      engineVersion: __APP_VERSION__,
      startedAtUnixMs: BigInt(Date.now()),
      seats: seats.map((seat, slot) => ({
        slot,
        player: {
          kind: seat.bot ? IdentityKind.BUILT_IN_BOT : IdentityKind.GUEST,
          id: seat.bot ? `bot-${slot}` : "you",
          displayName: seat.bot ? `Bot ${slot}` : playerName || "You",
        },
        botKind: seat.bot ?? undefined,
      })),
    });
    const mySlot = seats.findIndex((seat) => seat.bot === null);
    run(new LocalMatchSource(header, Math.max(0, mySlot)));
  }

  function startOnline(header: MatchHeader): void {
    const session = connection.session;
    if (!session) return;
    const seat = header.seats.find((candidate) => sameIdentity(candidate.player, connection.identity));
    run(new OnlineMatchSource(session, header, seat ? seat.slot : null));
  }

  function startReplay(bytes: Uint8Array): void {
    try {
      const source = new ReplayMatchSource(fromBinary(ReplayFileSchema, bytes));
      replay.value = source;
      run(source);
    } catch (error) {
      toasts.error(`Cannot open the replay: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  function close(): void {
    controller.value?.source.leave();
    controller.value?.dispose();
    controller.value = null;
    replay.value = null;
  }

  async function downloadReplay(): Promise<void> {
    const source = controller.value?.source;
    if (!source) return;
    try {
      const bytes = await source.replayFile();
      if (bytes) downloadBytes(bytes, `mini-games-${source.header.matchId}.mgreplay`);
    } catch (error) {
      toasts.error(error instanceof Error ? error.message : String(error));
    }
  }

  // Online matches start on the server's push.
  let detach: (() => void) | null = null;
  watch(
    () => connection.session,
    (session) => {
      detach?.();
      detach = session ? session.events.on("matchStarted", (header) => startOnline(header)) : null;
    },
    { immediate: true },
  );

  return { controller, replay, startLocal, startOnline, startReplay, close, downloadReplay };
});
