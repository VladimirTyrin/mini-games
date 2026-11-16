<script setup lang="ts">
import { computed } from "vue";

import ChipsField from "@/components/form/ChipsField.vue";
import { matchingPresetKey } from "@/components/form/presets";
import SegmentedField from "@/components/form/SegmentedField.vue";
import { useGameSettingsForm } from "@/components/form/settings-form";
import StepperField from "@/components/form/StepperField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { TicTacToeFirstPlayerMode } from "@/proto/minigames/v2/tictactoe_pb";

import { boardPresets } from "./board-presets";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "tictactoe");
const { value, audience } = form;

const sizePresets = computed(() => form.acceptedPresets(boardPresets));

// Locally seat 0 is always the person holding the device and seat 1 the opponent.
const firstPlayerOptions = computed(() => {
  const local = audience.value === "local";
  return [
    { value: TicTacToeFirstPlayerMode.HOST, label: local ? "You" : "Host" },
    { value: TicTacToeFirstPlayerMode.GUEST, label: local ? "Opponent" : "Guest" },
    { value: TicTacToeFirstPlayerMode.RANDOM, label: "Random" },
  ];
});
</script>

<template>
  <div v-if="value" class="divide-y divide-white/5">
    <ChipsField
      label="Board size"
      :options="sizePresets"
      custom-label="Custom"
      :model-value="matchingPresetKey(sizePresets, value)"
      @update:model-value="form.applyPreset(sizePresets, $event)"
    >
      <template #custom>
        <StepperField
          label="Width"
          v-bind="form.field('field_width')"
          :model-value="value.fieldWidth"
          @update:model-value="form.update({ fieldWidth: $event })"
        />
        <StepperField
          label="Height"
          v-bind="form.field('field_height')"
          :model-value="value.fieldHeight"
          @update:model-value="form.update({ fieldHeight: $event })"
        />
        <StepperField
          label="In a row to win"
          v-bind="form.field('win_count')"
          :model-value="value.winCount"
          @update:model-value="form.update({ winCount: $event })"
        />
      </template>
    </ChipsField>
    <SegmentedField
      label="First move"
      help="Plays X"
      stacked
      :options="firstPlayerOptions"
      :model-value="value.firstPlayer"
      @update:model-value="form.update({ firstPlayer: $event })"
    />
  </div>
</template>
