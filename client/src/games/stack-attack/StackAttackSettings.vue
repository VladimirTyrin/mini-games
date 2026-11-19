<script setup lang="ts">
import { computed } from "vue";

import ChipsField from "@/components/form/ChipsField.vue";
import { matchingPresetKey } from "@/components/form/presets";
import { useGameSettingsForm } from "@/components/form/settings-form";
import StepperField from "@/components/form/StepperField.vue";
import { t } from "@/i18n";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "stackAttack");
const value = form.value;
const text = computed(() => t.games.stackAttack.settings);

const sizePresets = computed(() =>
  form.acceptedPresets([
    {
      key: "small",
      label: t.common.boardSize.small,
      detail: "10×8",
      patch: { fieldWidth: 10, fieldHeight: 8 },
    },
    {
      key: "medium",
      label: t.common.boardSize.medium,
      detail: "15×10",
      patch: { fieldWidth: 15, fieldHeight: 10 },
    },
    {
      key: "large",
      label: t.common.boardSize.large,
      detail: "20×12",
      patch: { fieldWidth: 20, fieldHeight: 12 },
    },
  ]),
);
</script>

<template>
  <div v-if="value" class="divide-y divide-white/5">
    <ChipsField
      :label="t.common.boardSize.label"
      :help="text.boardSizeHelp"
      :options="sizePresets"
      :custom-label="t.common.boardSize.custom"
      :model-value="matchingPresetKey(sizePresets, value)"
      @update:model-value="form.applyPreset(sizePresets, $event)"
    >
      <template #custom>
        <StepperField
          :label="t.common.boardSize.width"
          v-bind="form.field('field_width')"
          :model-value="value.fieldWidth"
          @update:model-value="form.update({ fieldWidth: $event })"
        />
        <StepperField
          :label="t.common.boardSize.height"
          v-bind="form.field('field_height')"
          :model-value="value.fieldHeight"
          @update:model-value="form.update({ fieldHeight: $event })"
        />
      </template>
    </ChipsField>
  </div>
</template>
