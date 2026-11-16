<script setup lang="ts">
import { errorMessage } from "@/util/error-message";
import { computed, onMounted, ref, watch } from "vue";

import {
  GoogleSignInCancelled,
  googleSignInSupportedHere,
  renderGoogleButton,
  signInWithGoogleNatively,
} from "@/platform/google-sign-in";
import { isNativeApp } from "@/platform/native-app";

import GoogleSignInButton from "./GoogleSignInButton.vue";
import { useConnectionStore } from "@/stores/connection";
import { usePrefsStore } from "@/stores/prefs";

const connection = useConnectionStore();
const prefs = usePrefsStore();
const name = ref(prefs.guestName);
const googleButton = ref<HTMLElement | null>(null);
const googleError = ref<string | null>(null);

const info = computed(() => connection.serverInfo);
const googleClientId = computed(() => info.value?.googleClientId ?? "");
const busy = computed(() => connection.state === "connecting" || connection.state === "reconnecting");

async function mountGoogle(): Promise<void> {
  if (isNativeApp || !googleClientId.value || !googleButton.value) return;
  if (!googleSignInSupportedHere()) {
    googleError.value = "Google sign-in needs the https site (or the Android app).";
    return;
  }
  try {
    await renderGoogleButton(googleButton.value, googleClientId.value, (idToken) =>
      connection.signInWithGoogle(idToken),
    );
  } catch (error) {
    googleError.value = errorMessage(error);
  }
}

async function signInNatively(): Promise<void> {
  googleError.value = null;
  try {
    connection.signInWithGoogle(await signInWithGoogleNatively(googleClientId.value));
  } catch (error) {
    if (error instanceof GoogleSignInCancelled) return;
    googleError.value = errorMessage(error);
  }
}

onMounted(() => void connection.loadServerInfo());
watch([googleClientId, googleButton], () => void mountGoogle());
</script>

<template>
  <section class="space-y-4 rounded-xl bg-panel p-4">
    <p v-if="connection.serverError" class="text-sm text-rose-300">
      {{ connection.serverError }} ({{ prefs.serverUrl }}). Check the server address in Settings.
    </p>
    <p v-else-if="connection.state === 'outdated'" class="text-sm text-rose-300">
      This server runs a newer version. Please update the app.
    </p>
    <template v-else-if="info">
      <form v-if="info.guestsAllowed" class="flex gap-2" @submit.prevent="connection.signInAsGuest(name)">
        <input
          v-model="name"
          maxlength="24"
          placeholder="Your name"
          class="flex-1 rounded bg-panel-raised px-3 py-2"
          autocomplete="nickname"
        />
        <button class="rounded bg-emerald-600 px-4 py-2 font-semibold" :disabled="!name.trim() || busy">
          Play as guest
        </button>
      </form>
      <div v-if="googleClientId" class="space-y-1">
        <GoogleSignInButton v-if="isNativeApp" :disabled="busy" @click="signInNatively" />
        <div v-else ref="googleButton" />
        <p v-if="googleError" class="text-xs text-gray-400">{{ googleError }}</p>
      </div>
    </template>
    <p v-else class="text-sm text-gray-400">Contacting the server…</p>
  </section>
</template>
