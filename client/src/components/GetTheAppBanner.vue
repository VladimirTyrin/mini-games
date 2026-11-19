<script setup lang="ts">
import { ref } from "vue";

import { t } from "@/i18n";
import { ANDROID_APK_URL } from "@/platform/android-app";
import { storage } from "@/platform/storage";
import { isAppleMobile, isInstalledWebApp } from "@/platform/web-app-install";

const DISMISSED_KEY = "mini-games.get-the-app.dismissed";

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
      <span class="flex-1">{{ t.app.getTheApp.androidPitch }}</span>
      <a :href="ANDROID_APK_URL" class="whitespace-nowrap rounded-full bg-sky-600 px-4 py-2 font-semibold">{{
        t.app.getTheApp.getTheApp
      }}</a>
    </template>
    <span v-else class="flex-1">
      {{ t.app.getTheApp.iosTap }} <span class="font-semibold">{{ t.app.getTheApp.iosShare }}</span>
      {{ t.app.getTheApp.iosThen }}
      <span class="font-semibold">{{ t.app.getTheApp.iosAddToHomeScreen }}</span>{{
        t.app.getTheApp.iosWorksOffline
      }}
    </span>
    <button class="px-1 text-lg text-gray-400" :aria-label="t.app.notNow" @click="dismiss">✕</button>
  </div>
</template>
