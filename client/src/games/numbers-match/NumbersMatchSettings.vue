<script setup lang="ts">
import { computed } from "vue";

import SegmentedField from "@/components/form/SegmentedField.vue";
import { useGameSettingsForm } from "@/components/form/settings-form";
import StepperField from "@/components/form/StepperField.vue";
import { t } from "@/i18n";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";

import { hintModeOptions } from "./hint-mode-options";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "numbersMatch");
const value = form.value;
const text = computed(() => t.games.numbersMatch.settings);
const hintModes = computed(() => hintModeOptions());
</script>

<template>
  <div v-if="value" class="divide-y divide-white/5">
    <SegmentedField
      :label="text.hints"
      :help="text.hintsHelp"
      stacked
      :options="hintModes"
      :model-value="value.hintMode"
      @update:model-value="form.update({ hintMode: $event })"
    />
    <StepperField
      :label="text.refills"
      :help="text.refillsHelp"
      v-bind="form.field('refills')"
      :model-value="value.refills"
      @update:model-value="form.update({ refills: $event })"
    />
  </div>
</template>
