<script setup lang="ts">
import { computed } from "vue";

import ChipsField from "@/components/form/ChipsField.vue";
import MoreOptions from "@/components/form/MoreOptions.vue";
import { matchingPresetKey } from "@/components/form/presets";
import SegmentedField from "@/components/form/SegmentedField.vue";
import { useGameSettingsForm } from "@/components/form/settings-form";
import { SNAKE_SPEEDS } from "./speeds";
import StepperField from "@/components/form/StepperField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { SnakeDeadSnakeBehavior, SnakeWallCollisionMode } from "@/proto/minigames/v2/snake_pb";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "snake");
const value = form.value;

const sizePresets = computed(() =>
  form.acceptedPresets([
    { key: "small", label: "Small", detail: "12×12", patch: { fieldWidth: 12, fieldHeight: 12 } },
    { key: "medium", label: "Medium", detail: "15×15", patch: { fieldWidth: 15, fieldHeight: 15 } },
    { key: "large", label: "Large", detail: "20×20", patch: { fieldWidth: 20, fieldHeight: 20 } },
  ]),
);

const speedPresets = computed(() =>
  form.acceptedPresets(
    SNAKE_SPEEDS.map(({ key, label, tickIntervalMs }) => ({ key, label, patch: { tickIntervalMs } })),
  ),
);

// Chance per tick while below the cap, in per mille: 1000 refills on the next tick.
const foodPresets = computed(() =>
  form.acceptedPresets([
    { key: "instantly", label: "Instantly", patch: { foodSpawnPermille: 1000 } },
    { key: "often", label: "Often", patch: { foodSpawnPermille: 300 } },
    { key: "rarely", label: "Rarely", patch: { foodSpawnPermille: 50 } },
  ]),
);

const wallOptions = [
  { value: SnakeWallCollisionMode.DEATH, label: "Death" },
  { value: SnakeWallCollisionMode.WRAP_AROUND, label: "Wrap" },
] as const;

const deadSnakeOptions = [
  { value: SnakeDeadSnakeBehavior.DISAPPEAR, label: "Vanish" },
  { value: SnakeDeadSnakeBehavior.STAY_ON_FIELD, label: "Stay" },
] as const;
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
      </template>
    </ChipsField>
    <ChipsField
      label="Speed"
      :options="speedPresets"
      :unmatched-label="`Custom (${value.tickIntervalMs} ms)`"
      :error="form.field('tick_interval_ms').error"
      :model-value="matchingPresetKey(speedPresets, value)"
      @update:model-value="form.applyPreset(speedPresets, $event)"
    />
    <SegmentedField
      label="Walls"
      help="Wrap: leave one side, come back on the other"
      :options="wallOptions"
      :model-value="value.wallCollisionMode"
      @update:model-value="form.update({ wallCollisionMode: $event })"
    />
    <MoreOptions :has-problem="form.hasProblemIn(['max_food_count', 'food_spawn_permille'])">
      <SegmentedField
        label="Dead snakes"
        help="Stay: their bodies block the way"
        :options="deadSnakeOptions"
        :model-value="value.deadSnakeBehavior"
        @update:model-value="form.update({ deadSnakeBehavior: $event })"
      />
      <StepperField
        label="Food on field"
        help="Pieces out at the same time"
        v-bind="form.field('max_food_count')"
        :model-value="value.maxFoodCount"
        @update:model-value="form.update({ maxFoodCount: $event })"
      />
      <ChipsField
        label="Food appears"
        help="How soon a new piece shows up"
        :options="foodPresets"
        :unmatched-label="`Custom (${value.foodSpawnPermille / 10}%)`"
        :error="form.field('food_spawn_permille').error"
        :model-value="matchingPresetKey(foodPresets, value)"
        @update:model-value="form.applyPreset(foodPresets, $event)"
      />
    </MoreOptions>
  </div>
</template>
