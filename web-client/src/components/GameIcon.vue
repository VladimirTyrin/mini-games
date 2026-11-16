<script setup lang="ts">
import type { GamePlugin } from "@/games/game-plugin";

defineProps<{ plugin: GamePlugin }>();
</script>

<template>
  <component :is="plugin.Icon" v-if="plugin.Icon" class="shrink-0" />
  <span v-else class="flex shrink-0 items-center justify-center leading-none" aria-hidden="true">
    {{ plugin.icon }}
  </span>
</template>
