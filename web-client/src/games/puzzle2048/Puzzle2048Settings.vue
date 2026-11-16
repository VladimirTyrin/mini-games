<script setup lang="ts">
import { computed } from "vue";

import NumberField from "@/components/form/NumberField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import type { Puzzle2048Settings } from "@/proto/minigames/v2/puzzle2048_pb";

const settings = defineModel<GameSettings>({ required: true });

const value = computed(() => (settings.value.game.case === "puzzle2048" ? settings.value.game.value : null));

function update(patch: Partial<Omit<Puzzle2048Settings, "$typeName">>): void {
  if (!value.value) return;
  settings.value = { ...settings.value, game: { case: "puzzle2048", value: { ...value.value, ...patch } } };
}
</script>

<template>
  <div v-if="value" class="space-y-2">
    <NumberField label="Width" :model-value="value.width" @update:model-value="update({ width: $event })" />
    <NumberField
      label="Height"
      :model-value="value.height"
      @update:model-value="update({ height: $event })"
    />
    <NumberField
      label="Target tile"
      :model-value="value.target"
      @update:model-value="update({ target: $event })"
    />
  </div>
</template>
