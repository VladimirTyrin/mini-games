<script setup lang="ts">
import { t } from "@/i18n";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";

// Classic rules only: nothing to choose, so the form only says what they are.
defineModel<GameSettings>({ required: true });
</script>

<template>
  <div class="px-4 py-3 text-sm text-gray-300">
    <p>{{ t.games.seaBattle.settings.classicRules }}</p>
    <p class="mt-1 text-gray-400">{{ t.games.seaBattle.settings.noTouching }}</p>
  </div>
</template>
