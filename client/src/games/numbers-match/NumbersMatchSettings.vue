<script setup lang="ts">
import SegmentedField from "@/components/form/SegmentedField.vue";
import { useGameSettingsForm } from "@/components/form/settings-form";
import StepperField from "@/components/form/StepperField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";

import { hintModeOptions } from "./hint-mode-options";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "numbersMatch");
const value = form.value;
</script>

<template>
  <div v-if="value" class="divide-y divide-white/5">
    <SegmentedField
      label="Hints"
      help="Point out a pair you can take"
      stacked
      :options="hintModeOptions"
      :model-value="value.hintMode"
      @update:model-value="form.update({ hintMode: $event })"
    />
    <StepperField
      label="Refills"
      help="Copy the remaining numbers below"
      v-bind="form.field('refills')"
      :model-value="value.refills"
      @update:model-value="form.update({ refills: $event })"
    />
  </div>
</template>
