<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import AppHeader from "@/components/AppHeader.vue";
import Avatar from "@/components/Avatar.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import PresenceDot from "@/components/PresenceDot.vue";
import SignInPanel from "@/components/SignInPanel.vue";
import { friendLinkCode, friendLinkUrl, shareLink } from "@/platform/friend-links";
import { canNotifyWithoutAsking, pushAvailability } from "@/platform/push-notifications";
import type { Identity } from "@/proto/minigames/v2/identity_pb";
import { FriendLinkKind, Presence } from "@/proto/minigames/v2/social_pb";
import { useConnectionStore } from "@/stores/connection";
import { useFriendsStore } from "@/stores/friends";
import { usePrefsStore } from "@/stores/prefs";
import { useToastStore } from "@/stores/toasts";

const connection = useConnectionStore();
const friends = useFriendsStore();
const prefs = usePrefsStore();
const toasts = useToastStore();

const pasted = ref("");
const removing = ref<Identity | null>(null);
const notificationsOn = ref(true);
const availability = pushAvailability();
const notificationHint = computed(() => {
  switch (availability) {
    case "add-to-home-screen-first":
      return "To get invitations on iPhone, add this site to the Home Screen (Share, then Add to Home Screen) and open it from there.";
    case "blocked":
      return "Notifications are blocked for this site: allow them in the browser settings to get invitations while the app is closed.";
    default:
      return "Turn on notifications to get invitations while the app is closed.";
  }
});

/** Playing first, then online, then the rest; by name within each. */
const sortedFriends = computed(() => {
  const rank: Record<number, number> = { [Presence.PLAYING]: 0, [Presence.ONLINE]: 1 };
  return [...(friends.list?.friends ?? [])].sort(
    (a, b) =>
      (rank[a.presence] ?? 2) - (rank[b.presence] ?? 2) ||
      (a.identity?.displayName ?? "").localeCompare(b.identity?.displayName ?? ""),
  );
});

onMounted(async () => {
  if (friends.hasAccount) void friends.refresh();
  notificationsOn.value = await canNotifyWithoutAsking();
});

async function turnOnNotifications(): Promise<void> {
  notificationsOn.value = await friends.registerPush(true);
}

async function share(kind: FriendLinkKind): Promise<void> {
  friends.askForNotificationsOnce();
  const link = await friends.createLink(kind);
  if (!link) return;
  const url = friendLinkUrl(prefs.serverUrl, link.code);
  try {
    const how = await shareLink(url, "Add me as a friend in Mini Games");
    if (how === "copied") toasts.show("Link copied", "success");
  } catch {
    // Closing the share sheet is no error; the link stays in the list below.
  }
}

async function addFromPaste(): Promise<void> {
  const code = friendLinkCode(pasted.value);
  if (!code) {
    toasts.error("That is not a friend link");
    return;
  }
  pasted.value = "";
  await friends.useLink(code);
}

async function confirmRemove(): Promise<void> {
  const friend = removing.value;
  removing.value = null;
  if (friend) await friends.remove(friend.id).catch(() => undefined);
}

function expiry(unixMs: bigint): string {
  return new Date(Number(unixMs)).toLocaleDateString([], { day: "numeric", month: "short" });
}
</script>

<template>
  <AppHeader title="Friends" back="/online" />
  <main class="mx-auto w-full max-w-lg flex-1 space-y-4 p-4">
    <template v-if="!friends.hasAccount">
      <p class="rounded-xl bg-panel p-4 text-sm text-gray-300">
        {{
          friends.pendingLinkCode
            ? "Sign in with Google to add your friend."
            : "Friends need a Google account: sign in with Google to add them and invite them to games."
        }}
      </p>
      <SignInPanel v-if="!connection.isSignedIn" />
    </template>
    <template v-else>
      <section
        v-if="!notificationsOn && availability !== 'unsupported'"
        class="space-y-2 rounded-xl bg-panel p-4"
      >
        <p class="text-sm text-gray-300">
          {{ notificationHint }}
        </p>
        <button
          v-if="availability === 'available'"
          class="min-h-11 w-full rounded-lg bg-emerald-600 px-3 text-sm font-semibold"
          @click="turnOnNotifications"
        >
          Turn on notifications
        </button>
      </section>

      <section class="space-y-3 rounded-xl bg-panel p-4">
        <h2 class="font-semibold">Add friends</h2>
        <p class="text-xs text-gray-400">Links work for 7 days.</p>
        <div class="grid gap-2 sm:grid-cols-2">
          <button
            class="min-h-11 rounded-lg bg-emerald-600 px-3 text-sm font-semibold"
            @click="share(FriendLinkKind.SINGLE_USE)"
          >
            Link for one friend
          </button>
          <button
            class="min-h-11 rounded-lg bg-panel-raised px-3 text-sm font-semibold"
            @click="share(FriendLinkKind.MULTI_USE)"
          >
            Link for a group chat
          </button>
        </div>
        <form class="flex gap-2" @submit.prevent="addFromPaste">
          <input
            v-model="pasted"
            class="min-w-0 flex-1 rounded-lg bg-panel-raised px-3 py-2 text-sm"
            placeholder="Paste a friend's link"
            aria-label="Friend link"
            autocapitalize="off"
            spellcheck="false"
          />
          <button class="min-h-11 rounded-lg bg-panel-raised px-4 text-sm" :disabled="!pasted.trim()">
            Add
          </button>
        </form>
        <ul v-if="friends.list?.links.length" class="space-y-1">
          <li v-for="link in friends.list.links" :key="link.code" class="flex items-center gap-2 text-sm">
            <span class="min-w-0 flex-1 truncate text-gray-300">
              {{ link.kind === FriendLinkKind.MULTI_USE ? "Group link" : "One-friend link" }}
              · until {{ expiry(link.expiresAtUnixMs) }}
            </span>
            <button class="min-h-10 px-2 text-xs text-gray-400" @click="friends.revokeLink(link.code)">
              Revoke
            </button>
          </li>
        </ul>
      </section>

      <section v-if="friends.list?.incomingRequests.length" class="space-y-2 rounded-xl bg-panel p-4">
        <h2 class="font-semibold">Friend requests</h2>
        <ul class="space-y-2">
          <li
            v-for="request in friends.list.incomingRequests"
            :key="request.id"
            class="flex items-center gap-2"
          >
            <Avatar :url="request.avatarUrl" :name="request.displayName" :player-id="request.id" />
            <span class="min-w-0 flex-1 truncate text-sm">{{ request.displayName }}</span>
            <button
              class="min-h-10 rounded bg-emerald-600 px-3 text-sm"
              @click="friends.answerRequest(request.id, true)"
            >
              Accept
            </button>
            <button
              class="min-h-10 rounded bg-panel-raised px-3 text-sm"
              @click="friends.answerRequest(request.id, false)"
            >
              Decline
            </button>
          </li>
        </ul>
      </section>

      <section class="space-y-2 rounded-xl bg-panel p-4">
        <h2 class="font-semibold">Friends</h2>
        <p v-if="!sortedFriends.length" class="text-sm text-gray-400">
          No friends yet. Send someone a link, or tap “+ Friend” next to a player in a lobby.
        </p>
        <ul class="space-y-2">
          <li v-for="friend in sortedFriends" :key="friend.identity?.id" class="flex items-center gap-2">
            <Avatar
              :url="friend.identity?.avatarUrl ?? ''"
              :name="friend.identity?.displayName ?? ''"
              :player-id="friend.identity?.id"
            />
            <span class="min-w-0 flex-1">
              <span class="block truncate text-sm">{{ friend.identity?.displayName }}</span>
              <PresenceDot :presence="friend.presence" />
            </span>
            <button class="min-h-10 px-2 text-xs text-gray-400" @click="removing = friend.identity ?? null">
              Remove
            </button>
          </li>
        </ul>
        <template v-if="friends.list?.outgoingRequests.length">
          <h3 class="pt-2 text-xs uppercase tracking-wide text-gray-400">Waiting for an answer</h3>
          <ul class="space-y-2">
            <li
              v-for="request in friends.list.outgoingRequests"
              :key="request.id"
              class="flex items-center gap-2"
            >
              <Avatar :url="request.avatarUrl" :name="request.displayName" :player-id="request.id" />
              <span class="min-w-0 flex-1 truncate text-sm text-gray-300">{{ request.displayName }}</span>
              <button class="min-h-10 px-2 text-xs text-gray-400" @click="friends.remove(request.id)">
                Cancel
              </button>
            </li>
          </ul>
        </template>
      </section>
    </template>
  </main>
  <ConfirmDialog
    v-if="removing"
    :title="`Remove ${removing.displayName}?`"
    message="You will no longer see each other in your friend lists."
    confirm-label="Remove"
    cancel-label="Keep"
    @confirm="confirmRemove"
    @cancel="removing = null"
  />
</template>
