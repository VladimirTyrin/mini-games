<script setup lang="ts">
import { computed } from "vue";

import SegmentedField from "@/components/form/SegmentedField.vue";
import { useGameSettingsForm } from "@/components/form/settings-form";
import { CheckersWhitePlayer } from "@/proto/minigames/v2/checkers_pb";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "checkers");
const { value, audience } = form;

// Locally seat 0 is always the person holding the device and seat 1 the opponent.
const whiteOptions = computed(() => {
  const local = audience.value === "local";
  return [
    { value: CheckersWhitePlayer.HOST, label: local ? "You" : "Host" },
    { value: CheckersWhitePlayer.GUEST, label: local ? "Opponent" : "Guest" },
    { value: CheckersWhitePlayer.RANDOM, label: "Random" },
  ];
});
</script>

<template>
  <div v-if="value" class="divide-y divide-white/5">
    <SegmentedField
      label="Plays white"
      help="White moves first"
      stacked
      :options="whiteOptions"
      :model-value="value.whitePlayer"
      @update:model-value="form.update({ whitePlayer: $event })"
    />
  </div>
</template>
