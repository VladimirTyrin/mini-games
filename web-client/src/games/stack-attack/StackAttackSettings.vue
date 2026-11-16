<script setup lang="ts">
import { computed } from "vue";

import ChipsField from "@/components/form/ChipsField.vue";
import { matchingPresetKey } from "@/components/form/presets";
import { useGameSettingsForm } from "@/components/form/settings-form";
import StepperField from "@/components/form/StepperField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "stackAttack");
const value = form.value;

const sizePresets = computed(() =>
  form.acceptedPresets([
    { key: "small", label: "Small", detail: "10×8", patch: { fieldWidth: 10, fieldHeight: 8 } },
    { key: "medium", label: "Medium", detail: "15×10", patch: { fieldWidth: 15, fieldHeight: 10 } },
    { key: "large", label: "Large", detail: "20×12", patch: { fieldWidth: 20, fieldHeight: 12 } },
  ]),
);
</script>

<template>
  <div v-if="value" class="divide-y divide-white/5">
    <ChipsField
      label="Warehouse size"
      help="Wider means more crates to dodge"
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
      </template>
    </ChipsField>
  </div>
</template>
