<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";

import Avatar from "@/components/Avatar.vue";
import { pluginById } from "@/games";
import { useFriendsStore } from "@/stores/friends";

const friends = useFriendsStore();
const now = ref(Date.now());
// The countdown only needs whole seconds.
const ticker = setInterval(() => (now.value = Date.now()), 1000);
onBeforeUnmount(() => clearInterval(ticker));

/** The newest invitation that has not run out; older ones wait behind it. */
const invite = computed(() =>
  [...friends.incomingInvites].reverse().find((open) => Number(open.expiresAtUnixMs) > now.value),
);
const secondsLeft = computed(() =>
  invite.value ? Math.max(0, Math.floor((Number(invite.value.expiresAtUnixMs) - now.value) / 1000)) : 0,
);
const gameTitle = computed(() => (invite.value ? (pluginById(invite.value.game)?.title ?? "a game") : ""));
</script>

<template>
  <div
    v-if="invite"
    class="fixed inset-x-0 top-0 z-40 flex justify-center p-3"
    style="padding-top: max(0.75rem, env(safe-area-inset-top))"
    role="alertdialog"
    aria-live="assertive"
    :aria-label="`${invite.from?.displayName} invites you to ${gameTitle}`"
  >
    <div class="w-full max-w-md space-y-3 rounded-2xl bg-slate-800 p-4 shadow-xl ring-1 ring-white/10">
      <div class="flex items-center gap-3">
        <Avatar
          :url="invite.from?.avatarUrl ?? ''"
          :name="invite.from?.displayName ?? ''"
          :player-id="invite.from?.id"
        />
        <p class="min-w-0 flex-1 text-sm">
          <span class="font-semibold">{{ invite.from?.displayName }}</span>
          invites you to {{ gameTitle }}
        </p>
        <span class="text-xs text-gray-400 tabular-nums">{{ secondsLeft }} s</span>
      </div>
      <div class="flex gap-2">
        <button
          class="h-11 flex-1 rounded-xl bg-panel-raised font-medium"
          @click="friends.decline(invite.inviteId)"
        >
          Decline
        </button>
        <button
          class="h-11 flex-1 rounded-xl bg-emerald-600 font-semibold"
          @click="friends.accept(invite.inviteId, invite.lobbyId)"
        >
          Join
        </button>
      </div>
    </div>
  </div>
</template>
