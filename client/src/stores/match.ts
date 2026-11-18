import { create } from "@bufbuild/protobuf";
import { defineStore } from "pinia";
import { computed, shallowRef, watch } from "vue";

import type { BotKind, GameSettings } from "@/proto/minigames/v2/game_pb";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { type MatchHeader, MatchHeaderSchema } from "@/proto/minigames/v2/match_pb";
import { LocalMatchSource } from "@/local/local-match-source";
import { MatchController } from "@/match/match-controller";
import type { MatchSource } from "@/match/match-source";
import { OnlineMatchSource } from "@/match/online-match-source";
import { ReplayMatchSource } from "@/match/replay-match-source";
import { pluginBySettings } from "@/games";
import { downloadInBrowser } from "@/platform/download";
import { saveReplayFile } from "@/platform/saved-replays";
import { replayFileNameForMatch } from "@/replays/replay-file-name";

import { useConnectionStore } from "./connection";
import { useLobbyStore } from "./lobby";
import { useToastStore } from "./toasts";
import { errorMessage } from "@/util/error-message";
import { sameIdentity } from "@/util/players";

export interface LocalSeat {
  /** `null` for the local player. */
  bot: BotKind | null;
}

/** The match on screen (online, local or replay) and its controller. */
export const useMatchStore = defineStore("match", () => {
  const connection = useConnectionStore();
  const lobby = useLobbyStore();
  const toasts = useToastStore();
  const controller = shallowRef<MatchController | null>(null);
  const replay = shallowRef<ReplayMatchSource | null>(null);

  /** An online match on screen that has not ended: leaving the lobby would drop the seat. */
  const isInOnlineMatch = computed(
    () => controller.value?.source instanceof OnlineMatchSource && !controller.value.ended.value,
  );
  /** A match is being played that this build can follow. */
  const isPlaying = computed(() => controller.value !== null && !controller.value.incompatible.value);
  /**
   * Reloading for a new version now would lose something: a match being played, or the lobby
   * (settings draft, typed chat). A match this build cannot follow is better updated at once.
   */
  const holdsUpdates = computed(() =>
    controller.value ? !controller.value.incompatible.value : lobby.current !== null,
  );

  function run(source: MatchSource): MatchController {
    close();
    const created = new MatchController(source);
    controller.value = created;
    source.start();
    return created;
  }

  /** Offline match: you plus built-in bots, authority in a Web Worker. */
  function startLocal(picked: GameSettings, seats: LocalSeat[], playerName: string): void {
    const settings = pluginBySettings(picked.game.case)?.settingsForNewMatch?.(picked) ?? picked;
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
          // Bots count from 1 whether or not the person here holds a seat.
          displayName: seat.bot
            ? `Bot ${seats.slice(0, slot + 1).filter((earlier) => earlier.bot).length}`
            : playerName || "You",
          // The signed-in player's picture, so boards that show avatars show theirs offline too.
          avatarUrl: seat.bot ? "" : (connection.identity?.avatarUrl ?? ""),
        },
        botKind: seat.bot ?? undefined,
      })),
    });
    // No seat of one's own: watching bots play.
    const mySlot = seats.findIndex((seat) => seat.bot === null);
    run(new LocalMatchSource(header, mySlot < 0 ? null : mySlot));
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
      // After `run`: it closes the previous match first, which forgets the replay.
      run(source);
      replay.value = source;
    } catch (error) {
      toasts.error(`Cannot open the replay: ${errorMessage(error)}`);
    }
  }

  function close(): void {
    controller.value?.source.leave();
    controller.value?.dispose();
    controller.value = null;
    replay.value = null;
  }

  /** `gameTitle` names the file ("Snake 2026-11-16 14-05-33 3f9a1c.mgreplay"). */
  async function saveReplay(gameTitle: string): Promise<void> {
    const source = controller.value?.source;
    if (!source) return;
    try {
      const bytes = await source.replayFile();
      if (!bytes) {
        toasts.error("This match has no replay");
        return;
      }
      const fileName = replayFileNameForMatch(source.header, gameTitle);
      // The literal (see env.d.ts), so the web build drops the app's file storage.
      if (__NATIVE_APP__) {
        await saveReplayFile(bytes, fileName);
        toasts.show("Replay saved — find it under Saved replays on the home screen", "success", {
          label: "Show",
          to: "/replays",
        });
      } else {
        downloadInBrowser(bytes, fileName);
        toasts.show(`Replay saved to your downloads as ${fileName}`, "success");
      }
    } catch (error) {
      toasts.failure(error);
    }
  }

  // Online matches start on the server's push.
  let detach: (() => void) | null = null;
  watch(
    () => connection.session,
    (session) => {
      detach?.();
      const source = controller.value?.source;
      if (!session) {
        detach = null;
        if (source instanceof OnlineMatchSource) close();
        return;
      }
      if (source instanceof OnlineMatchSource) source.rebind(session);
      // Leaving the lobby as its match starts: the match is not this player's any more.
      const offStarted = session.events.on("matchStarted", (header) => {
        if (!lobby.isLeaving) void startOnline(header);
      });
      // A reload or reconnect into a running match: the server sends only a catch-up.
      const offCatchUp = session.events.on("catchUp", (catchUp) => {
        if (!catchUp.header || lobby.isLeaving) return;
        const shown = controller.value?.source;
        // A catch-up for the match on screen is that match's own business (its source applies
        // it); one for another match means the lobby moved on to it while this device was away.
        if (
          shown &&
          !(shown instanceof OnlineMatchSource && shown.header.matchId !== catchUp.header.matchId)
        ) {
          return;
        }
        startOnline(catchUp.header)?.applyCatchUp(catchUp);
      });
      // Kicked, or the lobby closed: its match is gone for this player.
      const offLobbyClosed = session.events.on("lobbyClosed", () => {
        if (controller.value?.source instanceof OnlineMatchSource) close();
      });
      // The server kept the seat no longer (away past its grace period): the match is gone.
      const offSignedIn = session.events.on("signedIn", ({ resumed }) => {
        if (resumed || !(controller.value?.source instanceof OnlineMatchSource)) return;
        close();
        toasts.show(
          connection.lostToRestart()
            ? "The server restarted for an update, and the match was stopped"
            : "You are no longer in the match you were in",
        );
      });
      detach = () => {
        offStarted();
        offCatchUp();
        offSignedIn();
        offLobbyClosed();
      };
    },
    { immediate: true },
  );

  return {
    controller,
    replay,
    isPlaying,
    isInOnlineMatch,
    holdsUpdates,
    startLocal,
    startOnline,
    startReplay,
    close,
    saveReplay,
  };
});
