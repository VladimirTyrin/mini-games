<script setup lang="ts">
import { errorMessage } from "@/util/error-message";
import { computed, onMounted, ref, watch } from "vue";

import { locale, t } from "@/i18n";
import {
  GoogleSignInCancelled,
  googleSignInSupportedHere,
  renderGoogleButton,
  signInWithGoogleNatively,
} from "@/platform/google-sign-in";

import GoogleSignInButton from "./GoogleSignInButton.vue";
import { useConnectionStore } from "@/stores/connection";
import { usePrefsStore } from "@/stores/prefs";

const connection = useConnectionStore();
const prefs = usePrefsStore();
const name = ref(prefs.guestName);
const googleButton = ref<HTMLElement | null>(null);
const googleError = ref<string | null>(null);
/** Google draws its button once its script has loaded; until then a placeholder holds the space. */
const googleButtonShown = ref(false);

const info = computed(() => connection.serverInfo);
const googleClientId = computed(() => info.value?.googleClientId ?? "");
const busy = computed(() => connection.state === "connecting" || connection.state === "reconnecting");
/** A session is (re)connecting by itself: a sign-in form now would look like being signed out. */
const connecting = computed(() => connection.session !== null && busy.value);

async function mountGoogle(): Promise<void> {
  if (__NATIVE_APP__ || !googleClientId.value || !googleButton.value) return;
  if (!googleSignInSupportedHere()) {
    googleError.value = t.app.signIn.googleNeedsHttps;
    return;
  }
  try {
    await renderGoogleButton(googleButton.value, googleClientId.value, locale.value, (idToken) =>
      connection.signInWithGoogle(idToken),
    );
    googleButtonShown.value = true;
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
// Google words its button itself: drawn again in the new language after a switch.
watch([googleClientId, googleButton, locale], () => void mountGoogle());

/** For the template, which cannot see the build-time constant. */
const isNativeApp = __NATIVE_APP__;
</script>

<template>
  <section class="space-y-4 rounded-xl bg-panel p-4">
    <div v-if="connection.serverError" class="space-y-3">
      <p class="text-sm text-rose-300">
        {{ t.app.signIn.serverUnreachable(connection.serverError, prefs.serverUrl) }}
      </p>
      <button
        type="button"
        class="min-h-11 rounded-lg bg-panel-raised px-4 py-2 text-sm"
        @click="connection.loadServerInfo()"
      >
        {{ t.common.tryAgain }}
      </button>
    </div>
    <p v-else-if="connection.state === 'outdated'" class="text-sm text-rose-300">
      {{ t.app.signIn.serverIsNewer }}
    </p>
    <p v-else-if="connecting" class="animate-pulse text-sm text-gray-300">
      {{
        connection.identity
          ? t.app.signIn.reconnectingAs(connection.identity.displayName)
          : t.app.signIn.signingIn
      }}
    </p>
    <template v-else-if="info">
      <form v-if="info.guestsAllowed" class="flex gap-2" @submit.prevent="connection.signInAsGuest(name)">
        <input
          v-model="name"
          data-testid="guest-name"
          maxlength="24"
          :placeholder="t.app.signIn.yourName"
          class="min-w-0 flex-1 rounded bg-panel-raised px-3 py-2"
          autocomplete="nickname"
        />
        <button
          class="shrink-0 rounded bg-emerald-600 px-4 py-2 font-semibold whitespace-nowrap"
          data-testid="play-as-guest"
          :disabled="!name.trim() || busy"
        >
          {{ t.app.signIn.playAsGuest }}
        </button>
      </form>
      <div v-if="googleClientId" class="space-y-1">
        <GoogleSignInButton v-if="isNativeApp" :disabled="busy" @click="signInNatively" />
        <div v-else class="relative h-11">
          <div
            v-if="!googleButtonShown && !googleError"
            class="absolute inset-0 animate-pulse rounded-full bg-panel-raised"
            aria-hidden="true"
          />
          <div ref="googleButton" class="w-full" />
        </div>
        <p v-if="googleError" class="text-xs text-gray-400">{{ googleError }}</p>
      </div>
    </template>
    <p v-else class="text-sm text-gray-400">{{ t.app.signIn.contactingServer }}</p>
  </section>
</template>
