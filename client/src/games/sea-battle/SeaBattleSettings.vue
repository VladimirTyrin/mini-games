<script setup lang="ts">
import type { GameSettings } from "@/proto/minigames/v2/game_pb";

// Classic rules only: nothing to choose, so the form only says what they are.
defineModel<GameSettings>({ required: true });
</script>

<template>
  <div class="px-4 py-3 text-sm text-gray-300">
    <p>
      Classic rules: a 10×10 sea and ten ships — one of four cells, two of three, three of two, four of one.
    </p>
    <p class="mt-1 text-gray-400">Ships may not touch, not even at a corner. A hit fires again.</p>
  </div>
</template>
