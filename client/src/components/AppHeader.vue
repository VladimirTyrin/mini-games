<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";

import Avatar from "@/components/Avatar.vue";
import { useBackHandler } from "@/platform/back-navigation";
import { useConnectionStore } from "@/stores/connection";

const props = defineProps<{ title: string; back?: string | (() => void) }>();
const router = useRouter();
const connection = useConnectionStore();

const status = computed(() => {
  switch (connection.state) {
    case "signed-in":
      return { text: connection.identity?.displayName ?? "online", tone: latencyTone(connection.latencyMs) };
    case "connecting":
    case "reconnecting":
      return { text: "connecting…", tone: "bg-amber-400" };
    case "outdated":
      return { text: "update the app", tone: "bg-rose-500" };
    case "replaced":
      return { text: "open in another window", tone: "bg-gray-500" };
    default:
      return null;
  }
});

/** Green while play feels instant, amber when a real-time game starts to lag, red beyond. */
function latencyTone(ms: number | null): string {
  if (ms === null || ms < 150) return "bg-emerald-400";
  if (ms < 400) return "bg-amber-400";
  return "bg-rose-500";
}

function goBack(): void {
  if (typeof props.back === "function") props.back();
  else if (props.back) void router.push(props.back);
}

if (props.back) useBackHandler(goBack);
</script>

<template>
  <header
    class="flex items-center gap-3 border-b border-white/5 px-4 py-3"
    style="padding-top: max(0.75rem, env(safe-area-inset-top))"
  >
    <button
      v-if="back"
      class="rounded px-2 py-1 text-lg hover:bg-panel-raised"
      aria-label="Back"
      @click="goBack"
    >
      ←
    </button>
    <h1 class="flex-1 truncate text-lg font-semibold">{{ title }}</h1>
    <slot />
    <span v-if="status" class="flex min-w-0 items-center gap-2 text-xs text-gray-400">
      <span class="h-2 w-2 shrink-0 rounded-full" :class="status.tone" />
      <span
        v-if="connection.state === 'signed-in' && connection.latencyMs !== null"
        class="shrink-0 tabular-nums"
        title="Round trip to the server"
        >{{ connection.latencyMs }} ms</span
      >
      <Avatar
        v-if="connection.state === 'signed-in' && connection.identity"
        :url="connection.identity.avatarUrl"
        :name="connection.identity.displayName"
        :player-id="connection.identity.id"
      />
      <!-- On a phone the avatar says who you are; the name would squeeze out the screen's title. -->
      <span
        class="truncate"
        :class="{ 'hidden sm:inline': connection.state === 'signed-in' && connection.identity }"
        >{{ status.text }}</span
      >
    </span>
  </header>
</template>
