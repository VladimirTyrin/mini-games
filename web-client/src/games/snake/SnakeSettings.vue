<script setup lang="ts">
import { computed } from "vue";

import NumberField from "@/components/form/NumberField.vue";
import SelectField from "@/components/form/SelectField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import {
  type SnakeSettings,
  SnakeDeadSnakeBehavior,
  SnakeWallCollisionMode,
} from "@/proto/minigames/v2/snake_pb";

const settings = defineModel<GameSettings>({ required: true });

const value = computed(() => (settings.value.game.case === "snake" ? settings.value.game.value : null));

function update(patch: Partial<Omit<SnakeSettings, "$typeName">>): void {
  if (!value.value) return;
  settings.value = { ...settings.value, game: { case: "snake", value: { ...value.value, ...patch } } };
}

const wallOptions = [
  { value: SnakeWallCollisionMode.DEATH, label: "Walls kill" },
  { value: SnakeWallCollisionMode.WRAP_AROUND, label: "Wrap around" },
] as const;

const deadSnakeOptions = [
  { value: SnakeDeadSnakeBehavior.DISAPPEAR, label: "Disappear" },
  { value: SnakeDeadSnakeBehavior.STAY_ON_FIELD, label: "Stay as obstacles" },
] as const;
</script>

<template>
  <div v-if="value" class="space-y-2">
    <NumberField
      label="Width"
      :model-value="value.fieldWidth"
      @update:model-value="update({ fieldWidth: $event })"
    />
    <NumberField
      label="Height"
      :model-value="value.fieldHeight"
      @update:model-value="update({ fieldHeight: $event })"
    />
    <SelectField
      label="Walls"
      :options="wallOptions"
      :model-value="value.wallCollisionMode"
      @update:model-value="update({ wallCollisionMode: $event })"
    />
    <SelectField
      label="Dead snakes"
      :options="deadSnakeOptions"
      :model-value="value.deadSnakeBehavior"
      @update:model-value="update({ deadSnakeBehavior: $event })"
    />
    <NumberField
      label="Max food on field"
      :model-value="value.maxFoodCount"
      @update:model-value="update({ maxFoodCount: $event })"
    />
    <NumberField
      label="Food spawn chance per tick"
      suffix="‰"
      :model-value="value.foodSpawnPermille"
      @update:model-value="update({ foodSpawnPermille: $event })"
    />
    <NumberField
      label="Tick"
      suffix="ms"
      :step="10"
      :model-value="value.tickIntervalMs"
      @update:model-value="update({ tickIntervalMs: $event })"
    />
  </div>
</template>
