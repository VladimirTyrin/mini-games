<script setup lang="ts">
import { computed } from "vue";
import { RouterLink } from "vue-router";

import FriendRow from "@/components/FriendRow.vue";
import { useInviteStatus } from "@/components/invite-status";
import { t } from "@/i18n";
import type { Friend } from "@/proto/minigames/v2/social_pb";
import { Presence } from "@/proto/minigames/v2/social_pb";
import { accountOf, useFriendsStore } from "@/stores/friends";
import { useLobbyStore } from "@/stores/lobby";

/** The friends at a glance; requests, links and removal stay on the Friends screen. */
const friends = useFriendsStore();
const lobby = useLobbyStore();
const { status, isPending, cancel } = useInviteStatus();

const inLobby = computed(() => new Set(lobby.members.map((member) => (member ? accountOf(member.id) : ""))));

/** From a lobby, an online friend not in it yet; the full list of whom to invite is in the lobby. */
function canInvite(friend: Friend): boolean {
  return (
    !!lobby.current &&
    !!friend.identity &&
    friend.presence === Presence.ONLINE &&
    !inLobby.value.has(friend.identity.id)
  );
}
</script>

<template>
  <section class="flex flex-col gap-2" data-testid="friends-panel">
    <div class="flex flex-wrap items-center gap-x-2">
      <h2 class="text-sm uppercase tracking-wide text-gray-400">{{ t.online.friends }}</h2>
      <RouterLink
        to="/friends"
        class="-my-2 ml-auto flex min-h-10 items-center gap-2 rounded px-2 text-sm text-emerald-300 hover:bg-white/5"
      >
        <template v-if="friends.incomingRequestCount">
          <span class="rounded-full bg-rose-600 px-2 text-xs font-semibold text-white" aria-hidden="true">
            {{ t.online.friendRequestCount(friends.incomingRequestCount) }}
          </span>
          <span class="sr-only">{{ t.online.friendRequestCountLabel(friends.incomingRequestCount) }}</span>
        </template>
        {{ `${friends.sortedFriends.length ? t.online.manageFriends : t.online.addFriends} ›` }}
      </RouterLink>
    </div>
    <p v-if="!friends.sortedFriends.length" class="rounded-xl bg-panel p-4 text-sm text-gray-400">
      {{ t.online.noFriendsYet }}
    </p>
    <!-- Scrolls within its own height, so a long list does not push the chat off the screen. -->
    <ul v-else class="max-h-[min(16rem,24vh)] space-y-2 overflow-y-auto rounded-xl bg-panel p-3">
      <FriendRow v-for="friend in friends.sortedFriends" :key="friend.identity?.id" :friend="friend">
        <template v-if="friend.identity && status(friend.identity.id)" #status>
          <span class="text-xs text-gray-300">{{ status(friend.identity.id) }}</span>
        </template>
        <button
          v-if="friend.identity && isPending(friend.identity.id)"
          class="min-h-10 rounded bg-panel-raised px-3 text-sm"
          @click="cancel(friend.identity.id)"
        >
          {{ t.common.cancel }}
        </button>
        <button
          v-else-if="canInvite(friend)"
          class="min-h-10 rounded bg-emerald-600 px-3 text-sm"
          @click="friends.invite(friend.identity!.id)"
        >
          {{ t.friends.invite.invite }}
        </button>
      </FriendRow>
    </ul>
  </section>
</template>
