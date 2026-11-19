<script setup lang="ts">
import { ANDROID_APK_URL } from "@/platform/android-app";
import { computed, onMounted, ref } from "vue";

import AppHeader from "@/components/AppHeader.vue";
import Avatar from "@/components/Avatar.vue";
import SignInPanel from "@/components/SignInPanel.vue";
import TestNotifications from "@/components/TestNotifications.vue";
import UpdateBanner from "@/components/UpdateBanner.vue";
import { LOCALES, type Locale, locale, setLocale, t } from "@/i18n";
import { normalizeServerUrl } from "@/platform/server-url";
import { nativeShellBuild } from "@/platform/web-bundle";
import { useAppUpdateStore } from "@/stores/app-update";
import { useConnectionStore } from "@/stores/connection";
import { usePrefsStore } from "@/stores/prefs";
import { useToastStore } from "@/stores/toasts";
import { privacyPolicyUrl } from "@/util/privacy-policy";

const prefs = usePrefsStore();
const connection = useConnectionStore();
const appUpdate = useAppUpdateStore();
const toasts = useToastStore();
const serverInput = ref(prefs.serverUrl);
/** Each language named in itself, so a player who cannot read the current one still finds theirs. */
const LANGUAGE_NAMES: Record<Locale, string> = { en: "English", ru: "Русский" };
const choosingLanguage = ref(false);
const editingServer = ref(false);
/** The server as a player recognizes it: its host, without the scheme and path. */
const serverHost = computed(() => {
  try {
    return new URL(prefs.serverUrl).host || prefs.serverUrl;
  } catch {
    return prefs.serverUrl;
  }
});

function pickLanguage(language: Locale): void {
  setLocale(language);
  choosingLanguage.value = false;
}
/** The app shell's build, when the running web bundle came over the air on top of it. */
const shellBuild = ref<number | null>(null);
// The build is 0 in a local build without MG_BUILD_NUMBER (deploy.sh and the Android build set it).
const version = computed(() => {
  if (!__NATIVE_APP__) return __APP_BUILD__ > 0 ? `${__APP_VERSION__} (${__APP_BUILD__})` : __APP_VERSION__;
  return shellBuild.value === null
    ? t.settings.versionWithWebBuild(__APP_VERSION__, __APP_BUILD__)
    : t.settings.versionWithBuilds(__APP_VERSION__, __APP_BUILD__, shellBuild.value);
});
const checkingForUpdates = ref(false);
/** For the template, which cannot see the build-time constant. */
const isNativeApp = __NATIVE_APP__;

onMounted(async () => {
  if (!__NATIVE_APP__) return;
  // The running web bundle may be newer than the app shell when it came over the air.
  shellBuild.value = await nativeShellBuild().catch(() => null);
});

async function checkForUpdates(): Promise<void> {
  checkingForUpdates.value = true;
  const report = await appUpdate.check();
  checkingForUpdates.value = false;
  switch (report.kind) {
    case "up-to-date":
      toasts.show(t.settings.upToDate, "success");
      break;
    case "web-bundle-ready":
      toasts.show(t.settings.updateDownloaded, "success");
      break;
    case "apk-available":
      toasts.show(t.settings.newAppVersion, "success");
      break;
    case "no-release-info":
    case "check-failed":
      toasts.error(t.settings.updateCheckFailed);
      break;
  }
}

function saveServer(): void {
  const url = normalizeServerUrl(serverInput.value);
  if (!url) {
    toasts.error(t.settings.notAServerAddress);
    return;
  }
  if (url !== prefs.serverUrl) {
    connection.disconnect();
    prefs.serverUrl = url;
    prefs.sessionToken = "";
  }
  serverInput.value = url;
  toasts.show(t.settings.serverSaved, "success");
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
  <AppHeader :title="t.settings.title" back="/" />
  <main class="mx-auto w-full max-w-lg flex-1 space-y-4 p-4">
    <section v-if="connection.isSignedIn" class="space-y-3 rounded-xl bg-panel p-4">
      <h2 class="font-semibold">{{ t.settings.account }}</h2>
      <div class="flex items-center gap-3">
        <Avatar
          :url="connection.identity?.avatarUrl ?? ''"
          :name="connection.identity?.displayName ?? ''"
          :player-id="connection.identity?.id"
        />
        <p class="flex-1 truncate text-sm text-gray-300">
          {{ t.settings.signedInAs(connection.identity?.displayName ?? "") }}
        </p>
        <button class="rounded bg-panel-raised px-3 py-2 text-sm" @click="connection.signOut()">
          {{ t.settings.signOut }}
        </button>
      </div>
    </section>
    <section v-else class="space-y-2">
      <h2 class="px-1 font-semibold">{{ t.settings.account }}</h2>
      <SignInPanel />
    </section>
    <TestNotifications v-if="connection.isSignedIn" />
    <!-- The language is picked once or twice: the current one, and its choices only on Change. -->
    <section class="space-y-3 rounded-xl bg-panel p-4">
      <div class="flex items-center gap-3">
        <h2 class="flex-1 font-semibold">{{ t.settings.language }}</h2>
        <span class="text-sm text-gray-300" :lang="locale">{{ LANGUAGE_NAMES[locale] }}</span>
        <button
          v-if="!choosingLanguage"
          class="rounded bg-panel-raised px-3 py-2 text-sm"
          data-testid="change-language"
          @click="choosingLanguage = true"
        >
          {{ t.settings.change }}
        </button>
      </div>
      <div v-if="choosingLanguage" class="flex gap-2">
        <button
          v-for="language in LOCALES"
          :key="language"
          :lang="language"
          class="min-h-10 flex-1 rounded px-3 text-sm"
          :class="language === locale ? 'bg-emerald-600 font-semibold' : 'bg-panel-raised'"
          :aria-pressed="language === locale"
          @click="pickLanguage(language)"
        >
          {{ LANGUAGE_NAMES[language] }}
        </button>
      </div>
    </section>
    <section v-if="!isNativeApp" class="flex items-center gap-3 rounded-xl bg-panel p-4">
      <div class="min-w-0 flex-1">
        <h2 class="font-semibold">{{ t.app.androidApp }}</h2>
        <p class="text-xs text-gray-400">{{ t.settings.androidAppPitch }}</p>
      </div>
      <a
        :href="ANDROID_APK_URL"
        class="rounded-lg border border-white/15 px-3 py-1.5 text-sm whitespace-nowrap text-gray-200 hover:bg-white/5"
        >{{ t.app.download }}</a
      >
    </section>
    <!-- Another server is for testing: folded, its address on the fold. -->
    <section class="rounded-xl bg-panel">
      <button
        type="button"
        class="flex min-h-12 w-full items-center gap-3 px-4 py-2 text-left text-sm text-gray-400 hover:text-gray-200"
        :aria-expanded="editingServer"
        data-testid="server-settings"
        @click="editingServer = !editingServer"
      >
        <span class="min-w-0 flex-1 break-words">{{ t.settings.serverShown(serverHost) }}</span>
        <span class="transition-transform" :class="editingServer ? 'rotate-180' : ''" aria-hidden="true">
          {{ "▾" }}
        </span>
      </button>
      <div v-if="editingServer" class="space-y-3 border-t border-white/5 p-4">
        <p class="text-xs text-gray-400">
          {{ t.settings.serverHint }}
        </p>
        <form class="flex gap-2" @submit.prevent="saveServer">
          <input
            v-model="serverInput"
            class="min-w-0 flex-1 rounded bg-panel-raised px-3 py-2 text-sm"
            autocapitalize="off"
            spellcheck="false"
          />
          <button class="rounded bg-emerald-600 px-3 text-sm">{{ t.common.save }}</button>
        </form>
        <button class="text-xs text-gray-400" @click="resetServer">{{ t.settings.resetToDefault }}</button>
      </div>
    </section>
    <UpdateBanner />
    <p class="text-center text-xs text-gray-500">{{ t.settings.version(version) }}</p>
    <p v-if="isNativeApp" class="text-center text-xs text-gray-500">
      <button class="underline hover:text-gray-300" :disabled="checkingForUpdates" @click="checkForUpdates">
        {{ checkingForUpdates ? t.settings.checkingForUpdates : t.settings.checkForUpdates }}
      </button>
    </p>
    <p class="text-center text-xs text-gray-500">
      <a :href="privacyPolicyUrl" class="underline hover:text-gray-300">{{ t.app.privacyPolicy }}</a>
    </p>
  </main>
</template>
