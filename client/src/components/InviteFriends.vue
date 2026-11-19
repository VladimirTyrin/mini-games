<script setup lang="ts">
import { computed, ref } from "vue";
import { RouterLink } from "vue-router";

import FriendRow from "@/components/FriendRow.vue";
import { useInviteStatus } from "@/components/invite-status";
import { t } from "@/i18n";
import type { Identity } from "@/proto/minigames/v2/identity_pb";
import { Presence } from "@/proto/minigames/v2/social_pb";
import { accountOf, useFriendsStore } from "@/stores/friends";

/** Who is in the lobby already: they need no invitation. */
const props = defineProps<{ members: (Identity | undefined)[] }>();

const friends = useFriendsStore();
const { status, isPending, cancel } = useInviteStatus();
const open = ref(false);

const inLobby = computed(() => new Set(props.members.map((member) => (member ? accountOf(member.id) : ""))));
/** Online friends first: an offline one can still be reached through a notification. */
const candidates = computed(() =>
  [...(friends.list?.friends ?? [])]
    .filter((friend) => friend.identity && !inLobby.value.has(friend.identity.id))
    .sort((a, b) => Number(b.presence === Presence.ONLINE) - Number(a.presence === Presence.ONLINE)),
);
</script>

<template>
  <section class="space-y-2 rounded-xl bg-panel p-4">
    <button
      class="flex min-h-10 w-full items-center justify-between text-left font-semibold"
      :aria-expanded="open"
      @click="open = !open"
    >
      {{ t.friends.invite.title }}
      <span class="text-sm text-gray-400">{{ open ? t.friends.invite.hide : t.friends.invite.show }}</span>
    </button>
    <template v-if="open">
      <p v-if="!friends.list?.friends.length" class="text-sm text-gray-400">
        {{ t.friends.invite.noFriendsYet }}
        <RouterLink to="/friends" class="text-emerald-300 underline">{{
          t.friends.invite.addFriends
        }}</RouterLink>
      </p>
      <p v-else-if="!candidates.length" class="text-sm text-gray-400">{{ t.friends.invite.allHere }}</p>
      <ul class="space-y-2">
        <FriendRow v-for="friend in candidates" :key="friend.identity!.id" :friend="friend">
          <template v-if="status(friend.identity!.id)" #status>
            <span class="text-xs text-gray-300">{{ status(friend.identity!.id) }}</span>
          </template>
          <button
            v-if="isPending(friend.identity!.id)"
            class="min-h-10 rounded bg-panel-raised px-3 text-sm"
            @click="cancel(friend.identity!.id)"
          >
            {{ t.common.cancel }}
          </button>
          <button
            v-else
            class="min-h-10 rounded bg-emerald-600 px-3 text-sm"
            @click="friends.invite(friend.identity!.id)"
          >
            {{ t.friends.invite.invite }}
          </button>
        </FriendRow>
      </ul>
    </template>
  </section>
</template>
