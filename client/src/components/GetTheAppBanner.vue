<script setup lang="ts">
import { ref } from "vue";

import { storage } from "@/platform/storage";
import { isAppleMobile, isInstalledWebApp } from "@/platform/web-app-install";

const DISMISSED_KEY = "mini-games.get-the-app.dismissed";
const APK_URL = "https://braintvsminigames.xyz/downloads/mini-games.apk";

/** What the device can install: the Android app, or the site itself on an iPhone home screen. */
type Offer = "android-app" | "ios-home-screen" | null;

function offerFor(): Offer {
  if (__NATIVE_APP__ || storage.get(DISMISSED_KEY) !== null) return null;
  const userAgent = navigator.userAgent;
  if (/android/i.test(userAgent)) return "android-app";
  return isAppleMobile() && !isInstalledWebApp() ? "ios-home-screen" : null;
}

const offer = ref<Offer>(offerFor());

function dismiss(): void {
  storage.set(DISMISSED_KEY, "1");
  offer.value = null;
}
</script>

<template>
  <div v-if="offer" class="flex items-center gap-3 rounded-xl bg-sky-900/50 p-3 text-sm">
    <template v-if="offer === 'android-app'">
      <span class="flex-1">The Android app: plays offline, updates itself.</span>
      <a :href="APK_URL" class="whitespace-nowrap rounded-full bg-sky-600 px-4 py-2 font-semibold"
        >Get the app</a
      >
    </template>
    <span v-else class="flex-1">
      Install it like an app: tap <span class="font-semibold">Share</span> ⎙, then
      <span class="font-semibold">Add to Home Screen</span>. It works offline too.
    </span>
    <button class="px-1 text-lg text-gray-400" aria-label="Not now" @click="dismiss">✕</button>
  </div>
</template>
