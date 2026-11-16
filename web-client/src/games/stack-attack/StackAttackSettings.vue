<script setup lang="ts">
import type { GameSettings } from "@/proto/minigames/v2/game_pb";

defineModel<GameSettings>({ required: true });
</script>

<template>
  <p class="text-sm text-gray-400">Default settings</p>
</template>
