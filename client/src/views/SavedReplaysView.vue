<script setup lang="ts">
import { onMounted, ref } from "vue";

import AppHeader from "@/components/AppHeader.vue";
import { gamePlugins } from "@/games";
import {
  deleteSavedReplay,
  listSavedReplays,
  readSavedReplay,
  shareSavedReplay,
} from "@/platform/saved-replays";
import type { SavedReplay } from "@/replays/saved-replay";
import { useMatchStore } from "@/stores/match";
import { useToastStore } from "@/stores/toasts";
import { errorMessage } from "@/util/error-message";
import { formatFileSize } from "@/util/file-size";

const match = useMatchStore();
const toasts = useToastStore();
/** `null` while the folder is being read. */
const replays = ref<SavedReplay[] | null>(null);
const confirmingDelete = ref<string | null>(null);

const iconOf = (replay: SavedReplay) =>
  gamePlugins.find((plugin) => plugin.title === replay.gameTitle)?.icon ?? "🎞";
const playedAtText = (replay: SavedReplay) =>
  replay.playedAt.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });

async function refresh(): Promise<void> {
  try {
    replays.value = await listSavedReplays();
  } catch (error) {
    replays.value = [];
    toasts.failure(error);
  }
}

async function play(replay: SavedReplay): Promise<void> {
  try {
    match.startReplay(await readSavedReplay(replay.fileName));
  } catch (error) {
    // Deleted or moved outside the app since the list was read.
    toasts.error(`Cannot open the replay: ${errorMessage(error)}`);
    await refresh();
  }
}

async function share(replay: SavedReplay): Promise<void> {
  try {
    await shareSavedReplay(replay.fileName);
  } catch (error) {
    toasts.failure(error);
  }
}

async function remove(replay: SavedReplay): Promise<void> {
  confirmingDelete.value = null;
  try {
    await deleteSavedReplay(replay.fileName);
    toasts.show("Replay deleted");
  } catch (error) {
    toasts.failure(error);
  }
  await refresh();
}

onMounted(refresh);
</script>

<template>
  <AppHeader title="Saved replays" back="/" />
  <main class="mx-auto w-full max-w-3xl flex-1 space-y-3 p-4">
    <p v-if="replays === null" class="text-center text-sm text-gray-400">Loading…</p>
    <div v-else-if="replays.length === 0" class="space-y-2 rounded-xl bg-panel p-6 text-center">
      <p class="font-semibold">No saved replays yet</p>
      <p class="text-sm text-gray-400">
        When a match ends, tap “Save replay” and it shows up here, ready to watch again or share.
      </p>
    </div>
    <ul v-else class="space-y-2">
      <li v-for="replay in replays" :key="replay.fileName" class="rounded-xl bg-panel">
        <div class="flex items-center gap-2 p-2">
          <button
            class="flex min-w-0 flex-1 items-center gap-3 rounded-lg p-2 text-left hover:bg-panel-raised"
            @click="play(replay)"
          >
            <span class="text-3xl">{{ iconOf(replay) }}</span>
            <span class="min-w-0">
              <span class="block truncate font-semibold">{{ replay.gameTitle ?? replay.fileName }}</span>
              <span class="block text-sm text-gray-400">
                {{ playedAtText(replay) }}
                <template v-if="replay.origin === 'imported'"> · Imported</template>
                · {{ formatFileSize(replay.sizeBytes) }}
              </span>
            </span>
          </button>
          <button class="rounded px-3 py-2 text-sm hover:bg-panel-raised" @click="share(replay)">
            Share
          </button>
          <button
            class="rounded px-3 py-2 text-sm text-rose-300 hover:bg-panel-raised"
            @click="confirmingDelete = replay.fileName"
          >
            Delete
          </button>
        </div>
        <div
          v-if="confirmingDelete === replay.fileName"
          class="flex items-center justify-end gap-2 border-t border-white/5 px-4 py-2 text-sm"
        >
          <span class="flex-1 text-gray-300">Delete this replay? This cannot be undone.</span>
          <button class="rounded bg-panel-raised px-3 py-1" @click="confirmingDelete = null">Keep</button>
          <button class="rounded bg-rose-700 px-3 py-1" @click="remove(replay)">Delete</button>
        </div>
      </li>
    </ul>
    <p class="text-center text-xs text-gray-500">
      Kept in the app's folder (Android/data/xyz.braintvsminigames.app/files/replays) and removed if you
      uninstall the app. Share a replay to keep a copy elsewhere.
    </p>
  </main>
</template>
