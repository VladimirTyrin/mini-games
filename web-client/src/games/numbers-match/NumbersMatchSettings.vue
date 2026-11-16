<script setup lang="ts">
import { computed } from "vue";

import NumberField from "@/components/form/NumberField.vue";
import SelectField from "@/components/form/SelectField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import type { NumbersMatchSettings } from "@/proto/minigames/v2/numbers_match_pb";

import { hintModeOptions } from "./hint-mode-options";

const settings = defineModel<GameSettings>({ required: true });

const value = computed(() =>
  settings.value.game.case === "numbersMatch" ? settings.value.game.value : null,
);

function update(patch: Partial<Omit<NumbersMatchSettings, "$typeName">>): void {
  if (!value.value) return;
  settings.value = { ...settings.value, game: { case: "numbersMatch", value: { ...value.value, ...patch } } };
}
</script>

<template>
  <div v-if="value" class="space-y-2">
    <SelectField
      label="Hints"
      :options="hintModeOptions"
      :model-value="value.hintMode"
      @update:model-value="update({ hintMode: $event })"
    />
    <NumberField
      label="Refills"
      :model-value="value.refills"
      @update:model-value="update({ refills: $event })"
    />
  </div>
</template>
