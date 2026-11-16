<script setup lang="ts">
import { ref } from "vue";

import AppHeader from "@/components/AppHeader.vue";
import Avatar from "@/components/Avatar.vue";
import SignInPanel from "@/components/SignInPanel.vue";
import UpdateBanner from "@/components/UpdateBanner.vue";
import { normalizeServerUrl } from "@/platform/server-url";
import { useAppUpdateStore } from "@/stores/app-update";
import { useConnectionStore } from "@/stores/connection";
import { usePrefsStore } from "@/stores/prefs";
import { useToastStore } from "@/stores/toasts";

const prefs = usePrefsStore();
const connection = useConnectionStore();
const appUpdate = useAppUpdateStore();
const toasts = useToastStore();
const serverInput = ref(prefs.serverUrl);
const version = __NATIVE_APP__ ? `${__APP_VERSION__} (${__APP_BUILD__})` : __APP_VERSION__;

function saveServer(): void {
  const url = normalizeServerUrl(serverInput.value);
  if (!url) {
    toasts.error("That does not look like a server address");
    return;
  }
  if (url !== prefs.serverUrl) {
    connection.disconnect();
    prefs.serverUrl = url;
    prefs.sessionToken = "";
  }
  serverInput.value = url;
  toasts.show("Server saved", "success");
  void connection.loadServerInfo();
  void appUpdate.check();
}

function resetServer(): void {
  prefs.resetServerUrl();
  serverInput.value = prefs.serverUrl;
  connection.disconnect();
}
</script>

<template>
  <AppHeader title="Settings" back="/" />
  <main class="mx-auto w-full max-w-lg flex-1 space-y-4 p-4">
    <section class="space-y-3 rounded-xl bg-panel p-4">
      <h2 class="font-semibold">Server</h2>
      <p class="text-xs text-gray-400">
        Address of the game server, e.g. braintvsminigames.xyz or 146.103.106.44.
      </p>
      <form class="flex gap-2" @submit.prevent="saveServer">
        <input
          v-model="serverInput"
          class="flex-1 rounded bg-panel-raised px-3 py-2 text-sm"
          autocapitalize="off"
          spellcheck="false"
        />
        <button class="rounded bg-emerald-600 px-3 text-sm">Save</button>
      </form>
      <button class="text-xs text-gray-400" @click="resetServer">Reset to default</button>
    </section>
    <section v-if="connection.isSignedIn" class="space-y-3 rounded-xl bg-panel p-4">
      <h2 class="font-semibold">Account</h2>
      <div class="flex items-center gap-3">
        <Avatar
          :url="connection.identity?.avatarUrl ?? ''"
          :name="connection.identity?.displayName ?? ''"
          :player-id="connection.identity?.id"
        />
        <p class="flex-1 truncate text-sm text-gray-300">
          Signed in as {{ connection.identity?.displayName }}
        </p>
        <button class="rounded bg-panel-raised px-3 py-2 text-sm" @click="connection.signOut()">
          Sign out
        </button>
      </div>
    </section>
    <section v-else class="space-y-2">
      <h2 class="px-1 font-semibold">Account</h2>
      <SignInPanel />
    </section>
    <UpdateBanner />
    <p class="text-center text-xs text-gray-500">Version {{ version }}</p>
    <p class="text-center text-xs text-gray-500">
      <a href="privacy.html" class="underline hover:text-gray-300">Privacy policy</a>
    </p>
  </main>
</template>
