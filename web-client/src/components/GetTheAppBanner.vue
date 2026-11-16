<script setup lang="ts">
import { ref } from "vue";

import { storage } from "@/platform/storage";

const DISMISSED_KEY = "mini-games.get-the-app.dismissed";
const APK_URL = "https://braintvsminigames.xyz/downloads/mini-games.apk";

// Only Android browsers: the app is an APK, there is nothing to offer elsewhere.
const shown = ref(
  !__NATIVE_APP__ && /android/i.test(navigator.userAgent) && storage.get(DISMISSED_KEY) === null,
);

function dismiss(): void {
  storage.set(DISMISSED_KEY, "1");
  shown.value = false;
}
</script>

<template>
  <div v-if="shown" class="flex items-center gap-3 rounded-xl bg-sky-900/50 p-3 text-sm">
    <span class="flex-1"
      >Smoother on your phone: get the Android app — it works offline and updates itself.</span
    >
    <a :href="APK_URL" class="rounded-full bg-sky-600 px-4 py-2 font-semibold whitespace-nowrap"
      >Get the app</a
    >
    <button class="px-1 text-lg text-gray-400" aria-label="Not now" @click="dismiss">✕</button>
  </div>
</template>
