<script setup lang="ts">
import GameIcon from "@/components/GameIcon.vue";
import { ref } from "vue";
import { useRouter } from "vue-router";

import AppHeader from "@/components/AppHeader.vue";
import GetTheAppBanner from "@/components/GetTheAppBanner.vue";
import UpdateBanner from "@/components/UpdateBanner.vue";
import { gamePlugins } from "@/games";
import { t } from "@/i18n";
import { ANDROID_APK_URL } from "@/platform/android-app";
import { useMatchStore } from "@/stores/match";
import { privacyPolicyUrl } from "@/util/privacy-policy";

const router = useRouter();
const match = useMatchStore();
const fileInput = ref<HTMLInputElement | null>(null);

async function openReplay(event: Event): Promise<void> {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) return;
  match.startReplay(new Uint8Array(await file.arrayBuffer()));
  (event.target as HTMLInputElement).value = "";
}
// Android does not know the .mgreplay extension and would hide every file behind that filter.
const replayFileTypes = __NATIVE_APP__ ? undefined : ".mgreplay";

/** For the template, which cannot see the build-time constant. */
const isNativeApp = __NATIVE_APP__;
/** Pictures beside the labels: the same in every language. */
const ICONS = { settings: "⚙", playOnline: "🌐", openReplay: "🎞", savedReplays: "📂" } as const;
</script>

<template>
  <AppHeader :title="t.app.name">
    <button class="rounded px-2 py-1 text-sm hover:bg-panel-raised" @click="router.push('/settings')">
      {{ ICONS.settings }} {{ t.settings.title }}
    </button>
  </AppHeader>
  <main class="mx-auto w-full max-w-3xl flex-1 space-y-6 p-4">
    <UpdateBanner />
    <GetTheAppBanner />
    <section>
      <h2 class="mb-3 text-sm uppercase tracking-wide text-gray-400">{{ t.home.playNow }}</h2>
      <!-- Phones get two compact tiles per row (icon and title); wider screens have room for the
           tagline. -->
      <div class="grid grid-cols-2 gap-3">
        <button
          v-for="plugin in gamePlugins"
          :key="plugin.id"
          class="flex flex-col items-center gap-2 rounded-xl bg-panel p-3 text-center hover:bg-panel-raised sm:flex-row sm:gap-4 sm:p-4 sm:text-left"
          @click="router.push({ name: 'play', params: { gameId: plugin.id } })"
        >
          <GameIcon :plugin="plugin" class="size-12 shrink-0 text-3xl" />
          <span class="min-w-0">
            <span class="block font-semibold leading-tight">{{ plugin.title }}</span>
            <span class="hidden text-sm text-gray-400 sm:block">{{ plugin.tagline }}</span>
          </span>
        </button>
      </div>
    </section>
    <section class="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <button
        class="rounded-xl bg-emerald-700 p-4 text-left font-semibold hover:bg-emerald-600"
        @click="router.push('/online')"
      >
        {{ ICONS.playOnline }} {{ t.home.playOnline }}
        <span class="block text-sm font-normal text-emerald-100">{{ t.home.playOnlineHint }}</span>
      </button>
      <button
        class="rounded-xl bg-panel p-4 text-left font-semibold hover:bg-panel-raised"
        @click="fileInput?.click()"
      >
        {{ ICONS.openReplay }} {{ t.home.openReplay }}
        <span class="block text-sm font-normal text-gray-400">{{ t.home.replayFileHint }}</span>
      </button>
      <button
        v-if="isNativeApp"
        class="rounded-xl bg-panel p-4 text-left font-semibold hover:bg-panel-raised"
        @click="router.push('/replays')"
      >
        {{ ICONS.savedReplays }} {{ t.replays.title }}
        <span class="block text-sm font-normal text-gray-400">{{ t.home.savedReplaysHint }}</span>
      </button>
      <input ref="fileInput" type="file" :accept="replayFileTypes" class="hidden" @change="openReplay" />
    </section>
    <p class="text-center text-xs text-gray-500">
      <a :href="privacyPolicyUrl" class="underline hover:text-gray-300">{{ t.app.privacyPolicy }}</a>
      <template v-if="!isNativeApp">
        ·
        <a :href="ANDROID_APK_URL" class="underline hover:text-gray-300">{{ t.app.androidApp }}</a>
      </template>
    </p>
  </main>
</template>
