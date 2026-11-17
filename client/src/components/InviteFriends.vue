<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { RouterLink } from "vue-router";

import Avatar from "@/components/Avatar.vue";
import PresenceDot from "@/components/PresenceDot.vue";
import type { Identity } from "@/proto/minigames/v2/identity_pb";
import { InviteState, Presence } from "@/proto/minigames/v2/social_pb";
import { accountOf, useFriendsStore } from "@/stores/friends";

/** Who is in the lobby already: they need no invitation. */
const props = defineProps<{ members: (Identity | undefined)[] }>();

const friends = useFriendsStore();
const open = ref(false);
const now = ref(Date.now());
const ticker = setInterval(() => (now.value = Date.now()), 1000);
onBeforeUnmount(() => clearInterval(ticker));

const inLobby = computed(() => new Set(props.members.map((member) => (member ? accountOf(member.id) : ""))));
/** Online friends first: an offline one can still be reached through a notification. */
const candidates = computed(() =>
  [...(friends.list?.friends ?? [])]
    .filter((friend) => friend.identity && !inLobby.value.has(friend.identity.id))
    .sort((a, b) => Number(b.presence === Presence.ONLINE) - Number(a.presence === Presence.ONLINE)),
);

/** The latest invitation this device sent to that account, if any. */
function invitationTo(accountId: string) {
  let latest = undefined;
  for (const update of friends.sentInvites.values()) {
    if (update.friend?.id === accountId) latest = update;
  }
  return latest;
}

/**
 * Pending until its time is up, even if the news of its end never arrived; accepted only for a
 * while, since a friend who accepted and never came is free to be asked again.
 */
function stateOf(accountId: string): InviteState | undefined {
  const update = invitationTo(accountId);
  const timeUp = update !== undefined && friends.deadlineOf(update.inviteId) <= now.value;
  if (update?.state === InviteState.PENDING && timeUp) return InviteState.EXPIRED;
  if (update?.state === InviteState.ACCEPTED && timeUp) return undefined;
  return update?.state;
}

function status(accountId: string): string | null {
  const update = invitationTo(accountId);
  switch (stateOf(accountId)) {
    case InviteState.PENDING: {
      const seconds = Math.max(
        0,
        Math.floor((friends.deadlineOf(update?.inviteId ?? "") - now.value) / 1000),
      );
      return `Waiting… ${seconds} s`;
    }
    case InviteState.ACCEPTED:
      return "Coming!";
    case InviteState.DECLINED:
      return "Declined";
    case InviteState.EXPIRED:
      return "No answer";
    default:
      return null;
  }
}

function isPending(accountId: string): boolean {
  return stateOf(accountId) === InviteState.PENDING;
}

function cancel(accountId: string): void {
  const update = invitationTo(accountId);
  if (update) void friends.cancelInvite(update.inviteId);
}
</script>

<template>
  <section class="space-y-2 rounded-xl bg-panel p-4">
    <button
      class="flex min-h-10 w-full items-center justify-between text-left font-semibold"
      :aria-expanded="open"
      @click="open = !open"
    >
      Invite friends
      <span class="text-sm text-gray-400">{{ open ? "Hide" : "Show" }}</span>
    </button>
    <template v-if="open">
      <p v-if="!friends.list?.friends.length" class="text-sm text-gray-400">
        No friends yet.
        <RouterLink to="/friends" class="text-emerald-300 underline">Add friends</RouterLink>
      </p>
      <p v-else-if="!candidates.length" class="text-sm text-gray-400">All your friends are here.</p>
      <ul class="space-y-2">
        <li v-for="friend in candidates" :key="friend.identity!.id" class="flex items-center gap-2">
          <Avatar
            :url="friend.identity!.avatarUrl"
            :name="friend.identity!.displayName"
            :player-id="friend.identity!.id"
          />
          <span class="min-w-0 flex-1">
            <span class="block truncate text-sm">{{ friend.identity!.displayName }}</span>
            <span v-if="status(friend.identity!.id)" class="text-xs text-gray-300">
              {{ status(friend.identity!.id) }}
            </span>
            <PresenceDot v-else :presence="friend.presence" />
          </span>
          <button
            v-if="isPending(friend.identity!.id)"
            class="min-h-10 rounded bg-panel-raised px-3 text-sm"
            @click="cancel(friend.identity!.id)"
          >
            Cancel
          </button>
          <button
            v-else
            class="min-h-10 rounded bg-emerald-600 px-3 text-sm"
            @click="friends.invite(friend.identity!.id)"
          >
            Invite
          </button>
        </li>
      </ul>
    </template>
  </section>
</template>
