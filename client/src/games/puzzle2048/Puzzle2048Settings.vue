<script setup lang="ts">
import { computed } from "vue";

import ChipsField from "@/components/form/ChipsField.vue";
import { matchingPresetKey } from "@/components/form/presets";
import { useGameSettingsForm } from "@/components/form/settings-form";
import StepperField from "@/components/form/StepperField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "puzzle2048");
const value = form.value;

const sizePresets = computed(() =>
  form.acceptedPresets([
    { key: "small", label: "Small", detail: "4×4", patch: { width: 4, height: 4 } },
    { key: "medium", label: "Medium", detail: "5×5", patch: { width: 5, height: 5 } },
    { key: "large", label: "Large", detail: "6×6", patch: { width: 6, height: 6 } },
  ]),
);

// The core accepts only powers of two; these are the ones worth playing for.
const targetPresets = computed(() =>
  form.acceptedPresets(
    [512, 1024, 2048, 4096].map((target) => ({
      key: String(target),
      label: String(target),
      patch: { target },
    })),
  ),
);
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
          v-bind="form.field('width')"
          :model-value="value.width"
          @update:model-value="form.update({ width: $event })"
        />
        <StepperField
          label="Height"
          v-bind="form.field('height')"
          :model-value="value.height"
          @update:model-value="form.update({ height: $event })"
        />
      </template>
    </ChipsField>
    <ChipsField
      label="Target tile"
      help="Reach it to win"
      :options="targetPresets"
      :unmatched-label="`Custom (${value.target})`"
      :error="form.field('target').error"
      :model-value="matchingPresetKey(targetPresets, value)"
      @update:model-value="form.applyPreset(targetPresets, $event)"
    />
  </div>
</template>
