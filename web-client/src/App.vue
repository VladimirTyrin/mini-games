<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useRouter } from "vue-router";

import ToastList from "@/components/ToastList.vue";
import { useConnectionStore } from "@/stores/connection";
import { useLobbyStore } from "@/stores/lobby";
import { useMatchStore } from "@/stores/match";

const router = useRouter();
const connection = useConnectionStore();
const lobby = useLobbyStore();
const match = useMatchStore();

// The only place that navigates on state changes: a running match wins, then the lobby.
watch(
  () => match.controller,
  (controller) => {
    if (controller) void router.push({ name: "match" });
    else if (lobby.current) void router.push({ name: "lobby" });
  },
);
watch(
  () => lobby.current?.lobbyId,
  (lobbyId, previous) => {
    if (match.controller) return;
    if (lobbyId && lobbyId !== previous) void router.push({ name: "lobby" });
    if (!lobbyId && router.currentRoute.value.name === "lobby") void router.push({ name: "online" });
  },
);

onMounted(() => connection.resumeStoredSession());
</script>

<template>
  <!-- Edge-to-edge on Android: keep content above the gesture bar. -->
  <div class="flex min-h-full flex-col" style="padding-bottom: env(safe-area-inset-bottom)">
    <RouterView />
    <ToastList />
  </div>
</template>
