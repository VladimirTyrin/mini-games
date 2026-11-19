<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";

import AppHeader from "@/components/AppHeader.vue";
import Avatar from "@/components/Avatar.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import PresenceDot from "@/components/PresenceDot.vue";
import SignInPanel from "@/components/SignInPanel.vue";
import { locale, t } from "@/i18n";
import { friendLinkCode, friendLinkUrl, shareLink } from "@/platform/friend-links";
import { canNotifyWithoutAsking, pushAvailability } from "@/platform/push-notifications";
import type { Identity } from "@/proto/minigames/v2/identity_pb";
import { FriendLinkKind, Presence } from "@/proto/minigames/v2/social_pb";
import { useConnectionStore } from "@/stores/connection";
import { useFriendsStore } from "@/stores/friends";
import { usePrefsStore } from "@/stores/prefs";
import { useToastStore } from "@/stores/toasts";

const router = useRouter();
/** Back where the player came from (a lobby, say); Online when this screen was the first one. */
function goBack(): void {
  if (window.history.state?.back) router.back();
  else void router.push("/online");
}
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
      return t.friends.iphoneNotificationsHint;
    case "blocked":
      return t.friends.notificationsBlockedHint;
    default:
      return t.friends.notificationsHint;
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
    const how = await shareLink(url, t.friends.shareText);
    if (how === "copied") toasts.show(t.friends.linkCopied, "success");
  } catch {
    // Closing the share sheet is no error; the link stays in the list below.
  }
}

async function addFromPaste(): Promise<void> {
  const code = friendLinkCode(pasted.value);
  if (!code) {
    toasts.error(t.friends.notAFriendLink);
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
  // Day and month only: a link lasts a week, and the line is narrow.
  return new Date(Number(unixMs)).toLocaleDateString(locale.value, { day: "numeric", month: "short" });
}
</script>

<template>
  <AppHeader :title="t.friends.title" :back="goBack" />
  <main class="mx-auto w-full max-w-lg flex-1 space-y-4 p-4">
    <template v-if="!friends.hasAccount">
      <p class="rounded-xl bg-panel p-4 text-sm text-gray-300">
        {{ friends.pendingLinkCode ? t.friends.needGoogleToAddFriend : t.friends.needGoogle }}
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
          {{ t.friends.turnOnNotifications }}
        </button>
      </section>

      <section class="space-y-3 rounded-xl bg-panel p-4">
        <h2 class="font-semibold">{{ t.friends.addFriends }}</h2>
        <p class="text-xs text-gray-400">{{ t.friends.linksLast }}</p>
        <div class="grid gap-2 sm:grid-cols-2">
          <button
            class="min-h-11 rounded-lg bg-emerald-600 px-3 text-sm font-semibold"
            @click="share(FriendLinkKind.SINGLE_USE)"
          >
            {{ t.friends.linkForOneFriend }}
          </button>
          <button
            class="min-h-11 rounded-lg bg-panel-raised px-3 text-sm font-semibold"
            @click="share(FriendLinkKind.MULTI_USE)"
          >
            {{ t.friends.linkForGroupChat }}
          </button>
        </div>
        <form class="flex gap-2" @submit.prevent="addFromPaste">
          <input
            v-model="pasted"
            class="min-w-0 flex-1 rounded-lg bg-panel-raised px-3 py-2 text-sm"
            :placeholder="t.friends.pasteLink"
            :aria-label="t.friends.friendLink"
            autocapitalize="off"
            spellcheck="false"
          />
          <button class="min-h-11 rounded-lg bg-panel-raised px-4 text-sm" :disabled="!pasted.trim()">
            {{ t.friends.add }}
          </button>
        </form>
        <ul v-if="friends.list?.links.length" class="space-y-1">
          <li v-for="link in friends.list.links" :key="link.code" class="flex items-center gap-2 text-sm">
            <span class="min-w-0 flex-1 truncate text-gray-300">
              {{ link.kind === FriendLinkKind.MULTI_USE ? t.friends.groupLink : t.friends.oneFriendLink }}
              · {{ t.friends.until(expiry(link.expiresAtUnixMs)) }}
            </span>
            <button class="min-h-10 px-2 text-xs text-gray-400" @click="friends.revokeLink(link.code)">
              {{ t.friends.revoke }}
            </button>
          </li>
        </ul>
      </section>

      <section v-if="friends.list?.incomingRequests.length" class="space-y-2 rounded-xl bg-panel p-4">
        <h2 class="font-semibold">{{ t.friends.friendRequests }}</h2>
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
              {{ t.friends.accept }}
            </button>
            <button
              class="min-h-10 rounded bg-panel-raised px-3 text-sm"
              @click="friends.answerRequest(request.id, false)"
            >
              {{ t.friends.decline }}
            </button>
          </li>
        </ul>
      </section>

      <section class="space-y-2 rounded-xl bg-panel p-4">
        <h2 class="font-semibold">{{ t.friends.title }}</h2>
        <p v-if="!sortedFriends.length" class="text-sm text-gray-400">
          {{ t.friends.noFriendsYet(`+ ${t.lobby.addFriend}`) }}
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
              {{ t.friends.remove }}
            </button>
          </li>
        </ul>
        <template v-if="friends.list?.outgoingRequests.length">
          <h3 class="pt-2 text-xs uppercase tracking-wide text-gray-400">
            {{ t.friends.waitingForAnswer }}
          </h3>
          <ul class="space-y-2">
            <li
              v-for="request in friends.list.outgoingRequests"
              :key="request.id"
              class="flex items-center gap-2"
            >
              <Avatar :url="request.avatarUrl" :name="request.displayName" :player-id="request.id" />
              <span class="min-w-0 flex-1 truncate text-sm text-gray-300">{{ request.displayName }}</span>
              <button class="min-h-10 px-2 text-xs text-gray-400" @click="friends.remove(request.id)">
                {{ t.common.cancel }}
              </button>
            </li>
          </ul>
        </template>
      </section>
    </template>
  </main>
  <ConfirmDialog
    v-if="removing"
    :title="t.friends.removeTitle(removing.displayName)"
    :message="t.friends.removeMessage"
    :confirm-label="t.friends.remove"
    :cancel-label="t.friends.keep"
    @confirm="confirmRemove"
    @cancel="removing = null"
  />
</template>
