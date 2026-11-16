<script setup lang="ts">
import GameIcon from "@/components/GameIcon.vue";
import { ref } from "vue";
import { useRouter } from "vue-router";

import AppHeader from "@/components/AppHeader.vue";
import GetTheAppBanner from "@/components/GetTheAppBanner.vue";
import UpdateBanner from "@/components/UpdateBanner.vue";
import { gamePlugins } from "@/games";
import { useMatchStore } from "@/stores/match";

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
</script>

<template>
  <AppHeader title="Mini Games">
    <button class="rounded px-2 py-1 text-sm hover:bg-panel-raised" @click="router.push('/settings')">
      ⚙ Settings
    </button>
  </AppHeader>
  <main class="mx-auto w-full max-w-3xl flex-1 space-y-6 p-4">
    <UpdateBanner />
    <GetTheAppBanner />
    <section>
      <h2 class="mb-3 text-sm uppercase tracking-wide text-gray-400">Play now — works offline</h2>
      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <button
          v-for="plugin in gamePlugins"
          :key="plugin.id"
          class="flex items-center gap-4 rounded-xl bg-panel p-4 text-left hover:bg-panel-raised"
          @click="router.push({ name: 'play', params: { gameId: plugin.id } })"
        >
          <GameIcon :plugin="plugin" class="size-12 text-3xl" />
          <span>
            <span class="block font-semibold">{{ plugin.title }}</span>
            <span class="block text-sm text-gray-400">{{ plugin.tagline }}</span>
          </span>
        </button>
      </div>
    </section>
    <section class="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <button
        class="rounded-xl bg-emerald-700 p-4 text-left font-semibold hover:bg-emerald-600"
        @click="router.push('/online')"
      >
        🌐 Play online
        <span class="block text-sm font-normal text-emerald-100">Lobbies, friends, chat</span>
      </button>
      <button
        class="rounded-xl bg-panel p-4 text-left font-semibold hover:bg-panel-raised"
        @click="fileInput?.click()"
      >
        🎞 Open a replay
        <span class="block text-sm font-normal text-gray-400">.mgreplay file</span>
      </button>
      <button
        v-if="isNativeApp"
        class="rounded-xl bg-panel p-4 text-left font-semibold hover:bg-panel-raised"
        @click="router.push('/replays')"
      >
        📂 Saved replays
        <span class="block text-sm font-normal text-gray-400">Watch, share or delete</span>
      </button>
      <input ref="fileInput" type="file" :accept="replayFileTypes" class="hidden" @change="openReplay" />
    </section>
    <p class="text-center text-xs text-gray-500">
      <a href="privacy.html" class="underline hover:text-gray-300">Privacy policy</a>
    </p>
  </main>
</template>
