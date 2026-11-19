<script setup lang="ts">
import GameIcon from "@/components/GameIcon.vue";
import { computed, ref } from "vue";
import { useRouter } from "vue-router";

import AppHeader from "@/components/AppHeader.vue";
import HowToPlay from "@/components/HowToPlay.vue";
import HowToPlayButton from "@/components/HowToPlayButton.vue";
import Avatar from "@/components/Avatar.vue";
import ChatPanel from "@/components/ChatPanel.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import InviteFriends from "@/components/InviteFriends.vue";
import SettingsEditor from "@/components/SettingsEditor.vue";
import { pluginBySettings } from "@/games";
import { lobbyDisplayName } from "@/games/lobby-name";
import { botName, botNumbers } from "@/games/seat-names";
import { t } from "@/i18n";
import { hasCounted } from "@/match/match-series";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { type Identity, IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { ChatScope, LobbyPhase } from "@/proto/minigames/v2/lobby_pb";
import { useConnectionStore } from "@/stores/connection";
import { accountOf, useFriendsStore } from "@/stores/friends";
import { useLobbyStore } from "@/stores/lobby";
import { useMatchStore } from "@/stores/match";
import { usePrefsStore } from "@/stores/prefs";
import { sameIdentity } from "@/util/players";
import { settingsProblemText } from "@/util/refusal-text";
import { type SettingsProblem, botKinds, gameCatalog, settingsProblem } from "@/wasm/core";

const router = useRouter();
const connection = useConnectionStore();
const lobby = useLobbyStore();
const match = useMatchStore();
const prefs = usePrefsStore();

const state = computed(() => lobby.current);
const plugin = computed(() => pluginBySettings(state.value?.settings?.game.case));
const helpOpen = ref(false);
const facts = computed(() =>
  plugin.value && state.value?.settings
    ? plugin.value.describeSettings(state.value.settings, state.value.seats.length)
    : [],
);
const info = computed(() => gameCatalog().find((game) => game.id === plugin.value?.id));
const availableBots = computed(() => (plugin.value && info.value?.hasBots ? botKinds(plugin.value.id) : []));
const mySeat = computed(() => (lobby.mySeatIndex >= 0 ? state.value?.seats[lobby.mySeatIndex] : undefined));
const humanSeats = computed(() => state.value?.seats.filter((seat) => !seat.botKind).length ?? 0);
/** Seats the settings must suit: everyone seated now (some rules depend on it). */
const seatsTaken = computed(() => Math.max(info.value?.minPlayers ?? 1, state.value?.seats.length ?? 0));
/** Settings that do not suit the table as seated now (e.g. a single-player mode with a guest). */
const startProblem = computed(() =>
  state.value?.settings ? settingsProblem(state.value.settings, seatsTaken.value) : null,
);
const canStart = computed(
  () =>
    lobby.isHost &&
    !!state.value &&
    (state.value.phase === LobbyPhase.OPEN || state.value.phase === LobbyPhase.POST_MATCH) &&
    state.value.seats.length >= (info.value?.minPlayers ?? 1),
);

const labelOf = (player: Identity) => lobby.memberLabels.get(player.id) ?? player.displayName;
const isMe = (player: Identity | undefined) => sameIdentity(player, connection.identity);
const friends = useFriendsStore();
/** Another Google player who is neither a friend nor asked yet. */
const canBefriend = (player: Identity | undefined) =>
  friends.hasAccount &&
  !!player &&
  !isMe(player) &&
  player.kind === IdentityKind.GOOGLE &&
  accountOf(player.id) !== accountOf(connection.identity?.id ?? "") &&
  !friends.friendAccounts.has(accountOf(player.id)) &&
  !friends.requestedAccounts.has(accountOf(player.id));

const canEditSettings = computed(
  () =>
    lobby.isHost && (state.value?.phase === LobbyPhase.OPEN || state.value?.phase === LobbyPhase.POST_MATCH),
);
/** Settings being edited by the host; `null` while not editing. */
const draft = ref<GameSettings | null>(null);
const draftProblem = ref<SettingsProblem | null>(null);

function editSettings(): void {
  draft.value = state.value?.settings ?? null;
}

async function saveSettings(): Promise<void> {
  const current = state.value;
  if (!draft.value || !current || !plugin.value) return;
  await lobby
    .updateSettings(draft.value, current.maxSeats)
    .then(() => {
      if (draft.value && plugin.value) prefs.rememberSettings(plugin.value.id, draft.value);
      draft.value = null;
    })
    .catch(() => undefined);
}

/** Nobody else is here: play locally instead (no ping, works offline). */
/** The score of the matches these players played in a row here. */
const series = computed(() => {
  const current = state.value?.series;
  return hasCounted(current) ? current : null;
});
const iVotedToPlayAgain = computed(() => state.value?.playAgainVotes.some((voter) => isMe(voter)) ?? false);
const startsLocally = computed(() => humanSeats.value === 1 && (state.value?.observers.length ?? 0) === 0);
/** The server refuses new matches while it waits to restart; a local one does not need it. */
const startPausedForUpdate = computed(() => connection.serverUpdating && !startsLocally.value);

async function start(): Promise<void> {
  const current = state.value;
  if (!current?.settings) return;
  if (startsLocally.value) {
    const seats = current.seats.map((seat) => ({ bot: seat.botKind ?? null }));
    const settings = current.settings;
    await lobby.leave();
    match.startLocal(settings, seats, connection.playerName);
    return;
  }
  await lobby.start().catch(() => undefined);
}

/** Others would notice: the host's leaving closes the lobby, a member's frees a seat. */
const confirmingLeave = ref(false);
/** The people (not bots) who stay; the first of them becomes the host if the host leaves. */
const othersInLobby = computed(() =>
  lobby.members.filter(
    (member): member is Identity => !!member && !isMe(member) && member.kind !== IdentityKind.BUILT_IN_BOT,
  ),
);
const leaveMessage = computed(() => {
  const [nextHost] = othersInLobby.value;
  if (!lobby.isHost || !nextHost) return t.lobby.seatGoesToNext;
  return t.lobby.becomesHost(labelOf(nextHost));
});
function requestLeave(): void {
  if (othersInLobby.value.length > 0) confirmingLeave.value = true;
  else void leave();
}

async function leave(): Promise<void> {
  confirmingLeave.value = false;
  await lobby.leave();
  void router.push("/online");
}

const kicking = ref<Identity | null>(null);
function kick(): void {
  const member = kicking.value;
  kicking.value = null;
  if (member) void lobby.kick(member.id);
}
</script>

<template>
  <AppHeader :title="state ? lobbyDisplayName(state) : t.lobby.title" :back="requestLeave">
    <HowToPlayButton v-if="plugin" @open="helpOpen = true" />
  </AppHeader>
  <HowToPlay v-if="helpOpen && plugin" :plugin="plugin" @close="helpOpen = false" />
  <ConfirmDialog
    v-if="confirmingLeave"
    :title="t.lobby.leaveTitle"
    :message="leaveMessage"
    :confirm-label="t.common.leave"
    :cancel-label="t.lobby.stay"
    @confirm="leave"
    @cancel="confirmingLeave = false"
  />
  <ConfirmDialog
    v-if="kicking"
    :title="t.lobby.removeMemberTitle(labelOf(kicking))"
    :message="t.lobby.removeMemberMessage"
    :confirm-label="t.lobby.remove"
    :cancel-label="t.common.cancel"
    @confirm="kick"
    @cancel="kicking = null"
  />
  <main
    v-if="state && plugin"
    class="mx-auto grid w-full max-w-4xl flex-1 gap-4 p-4 md:grid-cols-[1fr_20rem]"
  >
    <div class="space-y-4">
      <section class="rounded-2xl bg-panel p-4">
        <div class="mb-2 flex items-center gap-2">
          <GameIcon :plugin="plugin" class="size-9 text-2xl" />
          <span class="font-semibold">{{ plugin.title }}</span>
          <button
            v-if="canEditSettings && !draft"
            class="ml-auto h-11 rounded-lg px-3 text-sm text-emerald-300 hover:bg-panel-raised"
            @click="editSettings"
          >
            {{ t.lobby.editSettings }}
          </button>
        </div>
        <template v-if="draft && canEditSettings">
          <SettingsEditor
            v-model="draft"
            :plugin="plugin"
            :seats="seatsTaken"
            audience="online"
            @validity="draftProblem = $event"
          />
          <div class="flex gap-2">
            <button class="h-11 flex-1 rounded-lg bg-panel-raised text-sm" @click="draft = null">
              {{ t.common.cancel }}
            </button>
            <button
              class="h-11 flex-1 rounded-lg bg-emerald-600 text-sm font-semibold hover:bg-emerald-500 disabled:opacity-50"
              :disabled="!!draftProblem"
              @click="saveSettings"
            >
              {{ t.common.save }}
            </button>
          </div>
        </template>
        <ul v-else class="flex flex-wrap gap-2 text-xs text-gray-300">
          <li v-for="fact in facts" :key="fact" class="rounded bg-panel-raised px-2 py-1">{{ fact }}</li>
        </ul>
      </section>

      <section class="space-y-2 rounded-xl bg-panel p-4">
        <h2 class="text-sm text-gray-400">
          {{ t.lobby.players(state.seats.length, state.maxSeats) }}
          <template v-if="series?.draws"> · {{ t.match.series.draws(series.draws) }}</template>
        </h2>
        <div
          v-for="(seat, index) in state.seats"
          :key="index"
          class="flex items-center gap-2 rounded bg-panel-raised px-3 py-2"
        >
          <Avatar
            v-if="seat.occupant"
            :url="seat.occupant.avatarUrl"
            :name="seat.occupant.displayName"
            :player-id="seat.occupant.id"
            :bot="!!seat.botKind"
          />
          <span class="flex-1 truncate">
            <template v-if="seat.botKind">{{
              botName(plugin, seat.botKind, botNumbers(state.seats)[index] ?? 0)
            }}</template>
            <template v-else>{{ seat.occupant ? labelOf(seat.occupant) : "" }}</template>
            <span v-if="isMe(seat.occupant)" class="text-xs text-sky-300"> ({{ t.common.you }})</span>
            <span v-if="sameIdentity(seat.occupant, state.host)" class="text-xs text-amber-300">
              {{ t.lobby.host }}</span
            >
            <span v-if="seat.connectionLost" class="text-xs text-rose-300"> {{ t.lobby.reconnecting }}</span>
          </span>
          <span v-if="series" class="text-xs whitespace-nowrap text-gray-300">
            {{ t.match.series.wins(series.wins[index] ?? 0) }}
          </span>
          <button
            v-if="canBefriend(seat.occupant)"
            class="-my-1 min-h-9 rounded px-2 text-sm whitespace-nowrap text-emerald-300 hover:bg-white/5"
            @click="friends.sendRequest(seat.occupant!.id)"
          >
            + {{ t.lobby.addFriend }}
          </button>
          <span
            v-if="!seat.botKind"
            class="text-xs"
            :class="seat.ready ? 'text-emerald-300' : 'text-gray-500'"
          >
            {{ seat.ready ? t.lobby.seatReady : t.lobby.seatNotReady }}
          </span>
          <button
            v-if="lobby.isHost && seat.botKind"
            class="-my-1 min-h-9 rounded px-2 text-sm text-gray-400 hover:bg-white/5 hover:text-rose-300"
            @click="lobby.removeSeat(index)"
          >
            {{ t.lobby.remove }}
          </button>
          <button
            v-else-if="
              lobby.isHost &&
              seat.occupant &&
              !isMe(seat.occupant) &&
              seat.occupant.kind !== IdentityKind.BUILT_IN_BOT
            "
            class="-my-1 min-h-9 rounded px-2 text-sm text-gray-400 hover:bg-white/5 hover:text-rose-300"
            @click="kicking = seat.occupant"
          >
            {{ t.lobby.kick }}
          </button>
        </div>
        <div
          v-if="lobby.isHost && availableBots.length && state.seats.length < state.maxSeats"
          class="flex flex-wrap gap-2 pt-1"
        >
          <button
            v-for="kind in availableBots"
            :key="plugin.botLabel(kind)"
            class="min-h-10 rounded bg-panel-raised px-3 py-2 text-sm"
            data-testid="add-bot"
            @click="lobby.addBot(kind)"
          >
            + {{ plugin.botLabel(kind) }}
          </button>
        </div>
        <p v-if="state.observers.length" class="pt-2 text-xs text-gray-400">
          {{ t.lobby.watchers }}
          <template v-for="(observer, index) in state.observers" :key="observer.id">
            <span>{{ index > 0 ? ", " : "" }}{{ labelOf(observer) }}</span>
            <span v-if="isMe(observer)" class="text-sky-300"> ({{ t.common.you }})</span>
          </template>
        </p>
      </section>

      <InviteFriends v-if="friends.hasAccount" :members="lobby.members" />

      <section class="flex flex-wrap gap-2">
        <button
          v-if="mySeat && !lobby.isHost"
          class="rounded bg-panel px-4 py-2"
          @click="lobby.setReady(!mySeat.ready)"
        >
          {{ mySeat.ready ? t.lobby.notReady : t.lobby.ready }}
        </button>
        <button v-if="mySeat" class="rounded bg-panel px-4 py-2 text-sm" @click="lobby.switchToObserver()">
          {{ t.match.watchInstead }}
        </button>
        <button v-else class="rounded bg-panel px-4 py-2 text-sm" @click="lobby.switchToPlayer()">
          {{ t.match.takeSeat }}
        </button>
        <button
          v-if="state.phase === LobbyPhase.POST_MATCH && mySeat"
          class="rounded bg-emerald-700 px-4 py-2 whitespace-nowrap disabled:opacity-60"
          :disabled="iVotedToPlayAgain"
          @click="lobby.votePlayAgain()"
        >
          {{ iVotedToPlayAgain ? t.lobby.waitingForVotes : t.match.playAgain }} ·
          {{ state.playAgainVotes.length }}/{{ humanSeats }}
        </button>
        <p v-if="canStart && startPausedForUpdate" class="ml-auto text-sm text-amber-300">
          {{ t.lobby.serverUpdatingSoon }}
        </p>
        <p v-else-if="canStart && startProblem" class="ml-auto text-sm text-rose-300">
          {{ settingsProblemText(startProblem) }} {{ t.lobby.editSettingsToStart }}
        </p>
        <button
          v-if="canStart"
          class="rounded bg-emerald-600 px-6 py-2 font-semibold disabled:opacity-50"
          :class="startProblem || startPausedForUpdate ? '' : 'ml-auto'"
          :disabled="!!startProblem || startPausedForUpdate"
          @click="start"
        >
          {{ startsLocally ? t.lobby.startOffline : t.lobby.start }}
        </button>
      </section>
    </div>
    <ChatPanel
      :messages="lobby.lobbyChat"
      :players="lobby.members"
      @send="lobby.sendChat(ChatScope.LOBBY, $event)"
    />
  </main>
</template>
