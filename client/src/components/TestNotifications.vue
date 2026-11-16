<script setup lang="ts">
import { computed } from "vue";

import { pushAvailability } from "@/platform/push-notifications";
import { TEST_PUSH_DELAY_SECONDS, useNotificationsStore } from "@/stores/notifications";

const notifications = useNotificationsStore();
const availability = pushAvailability();

function seconds(from: number, to: number): string {
  return `${(Math.max(0, to - from) / 1000).toFixed(1)} s`;
}

function clockTime(at: number): string {
  return new Date(at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

/** Each step of the last test that the device could time. */
const timings = computed(() => {
  const test = notifications.lastTest;
  if (!test) return [];
  const lines: string[] = [];
  if (test.receivedAt !== null)
    lines.push(`Delivered ${seconds(test.sentAt, test.receivedAt)} after sending`);
  if (test.tappedAt !== null) {
    lines.push(`Tapped ${seconds(test.sentAt, test.tappedAt)} after sending`);
    lines.push(`App ready ${seconds(test.tappedAt, test.handledAt)} after the tap`);
  } else if (test.receivedAt === null) {
    lines.push(`Opened ${seconds(test.sentAt, test.handledAt)} after sending`);
  }
  return lines;
});
</script>

<template>
  <section class="space-y-3 rounded-xl bg-panel p-4">
    <h2 class="font-semibold">Notifications</h2>
    <p v-if="availability === 'add-to-home-screen-first'" class="text-sm text-gray-400">
      On iPhone, notifications work in the app added to the Home Screen: tap Share, then “Add to Home Screen”,
      and open it from there.
    </p>
    <p v-else-if="availability === 'unsupported'" class="text-sm text-gray-400">
      This browser cannot receive notifications.
    </p>
    <p v-else-if="availability === 'blocked'" class="text-sm text-gray-400">
      Notifications are blocked for this site; allow them in the browser settings.
    </p>
    <template v-else>
      <p class="text-xs text-gray-400">
        Sends a test notification {{ TEST_PUSH_DELAY_SECONDS }} s after the tap, so you can close the app or
        lock the phone first.
      </p>
      <button
        class="min-h-11 w-full rounded bg-emerald-600 px-3 py-2 text-sm font-semibold disabled:opacity-50"
        :disabled="notifications.testStatus.kind === 'registering'"
        @click="notifications.sendTest()"
      >
        Send a test notification
      </button>
      <p v-if="notifications.testStatus.kind === 'registering'" class="text-sm text-gray-300">Setting up…</p>
      <p v-else-if="notifications.testStatus.kind === 'scheduled'" class="text-sm text-emerald-300">
        Arrives about {{ clockTime(notifications.testStatus.sendsAt) }}. Close the app now.
      </p>
      <p v-else-if="notifications.testStatus.kind === 'failed'" class="text-sm text-red-300">
        {{ notifications.testStatus.message }}
      </p>
    </template>
    <div v-if="notifications.lastTest" class="space-y-1 text-sm text-gray-300">
      <p class="text-xs text-gray-400">Last test, sent at {{ clockTime(notifications.lastTest.sentAt) }}:</p>
      <p v-for="line in timings" :key="line">{{ line }}</p>
    </div>
  </section>
</template>
