<script setup lang="ts">
import { ANDROID_APK_URL } from "@/platform/android-app";
import { onMounted, ref } from "vue";

import AppHeader from "@/components/AppHeader.vue";
import Avatar from "@/components/Avatar.vue";
import SignInPanel from "@/components/SignInPanel.vue";
import TestNotifications from "@/components/TestNotifications.vue";
import UpdateBanner from "@/components/UpdateBanner.vue";
import { normalizeServerUrl } from "@/platform/server-url";
import { nativeShellBuild } from "@/platform/web-bundle";
import { useAppUpdateStore } from "@/stores/app-update";
import { useConnectionStore } from "@/stores/connection";
import { usePrefsStore } from "@/stores/prefs";
import { useToastStore } from "@/stores/toasts";

const prefs = usePrefsStore();
const connection = useConnectionStore();
const appUpdate = useAppUpdateStore();
const toasts = useToastStore();
const serverInput = ref(prefs.serverUrl);
// The build is 0 in a local build without MG_BUILD_NUMBER (deploy.sh and the Android build set it).
const webVersion = __APP_BUILD__ > 0 ? `${__APP_VERSION__} (${__APP_BUILD__})` : __APP_VERSION__;
const version = ref(__NATIVE_APP__ ? `${__APP_VERSION__} (web ${__APP_BUILD__})` : webVersion);
const checkingForUpdates = ref(false);
/** For the template, which cannot see the build-time constant. */
const isNativeApp = __NATIVE_APP__;

onMounted(async () => {
  if (!__NATIVE_APP__) return;
  // The running web bundle may be newer than the app shell when it came over the air.
  const shellBuild = await nativeShellBuild().catch(() => null);
  if (shellBuild !== null) version.value = `${__APP_VERSION__} (web ${__APP_BUILD__}, app ${shellBuild})`;
});

async function checkForUpdates(): Promise<void> {
  checkingForUpdates.value = true;
  const report = await appUpdate.check();
  checkingForUpdates.value = false;
  switch (report.kind) {
    case "up-to-date":
      toasts.show("Up to date", "success");
      break;
    case "web-bundle-ready":
      toasts.show("Update downloaded — it starts once you leave the match or lobby", "success");
      break;
    case "apk-available":
      toasts.show("A new app version is available — tap Download", "success");
      break;
    case "no-release-info":
    case "check-failed":
      toasts.error("Could not check for updates — is the server reachable?");
      break;
  }
}

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
  const previous = prefs.serverUrl;
  prefs.resetServerUrl();
  serverInput.value = prefs.serverUrl;
  // Already the default: keep the session instead of signing in again.
  if (prefs.serverUrl !== previous) connection.disconnect();
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
    <TestNotifications v-if="connection.isSignedIn" />
    <section v-if="!isNativeApp" class="flex items-center gap-3 rounded-xl bg-panel p-4">
      <div class="min-w-0 flex-1">
        <h2 class="font-semibold">Android app</h2>
        <p class="text-xs text-gray-400">Plays offline and updates itself.</p>
      </div>
      <a
        :href="ANDROID_APK_URL"
        class="rounded-lg border border-white/15 px-3 py-1.5 text-sm whitespace-nowrap text-gray-200 hover:bg-white/5"
        >Download</a
      >
    </section>
    <UpdateBanner />
    <p class="text-center text-xs text-gray-500">Version {{ version }}</p>
    <p v-if="isNativeApp" class="text-center text-xs text-gray-500">
      <button class="underline hover:text-gray-300" :disabled="checkingForUpdates" @click="checkForUpdates">
        {{ checkingForUpdates ? "Checking for updates…" : "Check for updates" }}
      </button>
    </p>
    <p class="text-center text-xs text-gray-500">
      <a href="privacy.html" class="underline hover:text-gray-300">Privacy policy</a>
    </p>
  </main>
</template>
