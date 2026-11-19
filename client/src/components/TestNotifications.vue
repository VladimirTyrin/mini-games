<script setup lang="ts">
import { computed } from "vue";

import { formatTimeWithSeconds, t } from "@/i18n";
import { pushAvailability } from "@/platform/push-notifications";
import { TEST_PUSH_DELAY_SECONDS, useNotificationsStore } from "@/stores/notifications";
import { formatOneDecimal } from "@/util/format-decimal";

const notifications = useNotificationsStore();
const availability = pushAvailability();

function seconds(from: number, to: number): string {
  return t.app.units.seconds(formatOneDecimal(Math.max(0, to - from) / 1000));
}

function clockTime(at: number): string {
  return formatTimeWithSeconds(new Date(at));
}

/** Each step of the last test that the device could time. */
const timings = computed(() => {
  const test = notifications.lastTest;
  if (!test) return [];
  const lines: string[] = [];
  const texts = t.settings.notifications;
  if (test.receivedAt !== null) lines.push(texts.delivered(seconds(test.sentAt, test.receivedAt)));
  if (test.tappedAt !== null) {
    lines.push(texts.tapped(seconds(test.sentAt, test.tappedAt)));
    lines.push(texts.appReady(seconds(test.tappedAt, test.handledAt)));
  } else if (test.receivedAt === null) {
    lines.push(texts.opened(seconds(test.sentAt, test.handledAt)));
  }
  return lines;
});
</script>

<template>
  <section class="space-y-3 rounded-xl bg-panel p-4">
    <h2 class="font-semibold">{{ t.settings.notifications.title }}</h2>
    <p v-if="availability === 'add-to-home-screen-first'" class="text-sm text-gray-400">
      {{ t.settings.notifications.iphoneHint }}
    </p>
    <p v-else-if="availability === 'unsupported'" class="text-sm text-gray-400">
      {{ t.settings.notifications.unsupported }}
    </p>
    <p v-else-if="availability === 'blocked'" class="text-sm text-gray-400">
      {{ t.settings.notifications.blocked }}
    </p>
    <template v-else>
      <p class="text-xs text-gray-400">
        {{ t.settings.notifications.testHint(TEST_PUSH_DELAY_SECONDS) }}
      </p>
      <button
        type="button"
        class="rounded-lg border border-white/15 px-3 py-1.5 text-sm whitespace-nowrap text-gray-200 hover:bg-white/5 disabled:opacity-50"
        :disabled="notifications.testStatus.kind === 'registering'"
        @click="notifications.sendTest()"
      >
        {{ t.settings.notifications.sendTest }}
      </button>
      <p v-if="notifications.testStatus.kind === 'registering'" class="text-sm text-gray-300">
        {{ t.settings.notifications.settingUp }}
      </p>
      <p v-else-if="notifications.testStatus.kind === 'scheduled'" class="text-sm text-emerald-300">
        {{ t.settings.notifications.arrivesAt(clockTime(notifications.testStatus.sendsAt)) }}
      </p>
      <p v-else-if="notifications.testStatus.kind === 'failed'" class="text-sm text-red-300">
        {{ notifications.testStatus.message }}
      </p>
    </template>
    <div v-if="notifications.lastTest" class="space-y-1 text-sm text-gray-300">
      <p class="text-xs text-gray-400">
        {{ t.settings.notifications.lastTestSentAt(clockTime(notifications.lastTest.sentAt)) }}
      </p>
      <p v-for="line in timings" :key="line">{{ line }}</p>
    </div>
  </section>
</template>
