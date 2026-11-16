import { create } from "@bufbuild/protobuf";
import { defineStore } from "pinia";
import { shallowRef, watch } from "vue";

import type { BotKind, GameSettings } from "@/proto/minigames/v2/game_pb";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { type MatchHeader, MatchHeaderSchema } from "@/proto/minigames/v2/match_pb";
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

  function run(source: MatchSource): MatchController {
    close();
    const created = new MatchController(source);
    controller.value = created;
    source.start();
    return created;
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

  function startOnline(header: MatchHeader): MatchController | null {
    const session = connection.session;
    if (!session) return null;
    const seat = header.seats.find((candidate) => sameIdentity(candidate.player, connection.identity));
    return run(new OnlineMatchSource(session, header, seat ? seat.slot : null));
  }

  function startReplay(bytes: Uint8Array): void {
    try {
      const source = new ReplayMatchSource(bytes);
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
      if (!session) {
        detach = null;
        return;
      }
      const offStarted = session.events.on("matchStarted", (header) => void startOnline(header));
      // A reload or reconnect into a running match: the server sends only a catch-up.
      const offCatchUp = session.events.on("catchUp", (catchUp) => {
        if (controller.value || !catchUp.header) return;
        startOnline(catchUp.header)?.applyCatchUp(catchUp);
      });
      detach = () => {
        offStarted();
        offCatchUp();
      };
    },
    { immediate: true },
  );

  return { controller, replay, startLocal, startOnline, startReplay, close, downloadReplay };
});
