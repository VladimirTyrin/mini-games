<script setup lang="ts">
import { computed } from "vue";

import ChipsField from "@/components/form/ChipsField.vue";
import MoreOptions from "@/components/form/MoreOptions.vue";
import { matchingPresetKey } from "@/components/form/presets";
import SegmentedField from "@/components/form/SegmentedField.vue";
import { useGameSettingsForm } from "@/components/form/settings-form";
import { t } from "@/i18n";
import { SNAKE_SPEEDS } from "./speeds";
import StepperField from "@/components/form/StepperField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { SnakeDeadSnakeBehavior, SnakeWallCollisionMode } from "@/proto/minigames/v2/snake_pb";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "snake");
const value = form.value;
const text = computed(() => t.games.snake.settings);

const sizePresets = computed(() =>
  form.acceptedPresets([
    {
      key: "small",
      label: t.common.boardSize.small,
      detail: "12×12",
      patch: { fieldWidth: 12, fieldHeight: 12 },
    },
    {
      key: "medium",
      label: t.common.boardSize.medium,
      detail: "15×15",
      patch: { fieldWidth: 15, fieldHeight: 15 },
    },
    {
      key: "large",
      label: t.common.boardSize.large,
      detail: "20×20",
      patch: { fieldWidth: 20, fieldHeight: 20 },
    },
  ]),
);

const speedPresets = computed(() =>
  form.acceptedPresets(
    SNAKE_SPEEDS.map(({ key, tickIntervalMs }) => ({
      key,
      label: t.games.snake.speeds[key],
      patch: { tickIntervalMs },
    })),
  ),
);

// Chance per tick while below the cap, in per mille: 1000 refills on the next tick.
const foodPresets = computed(() =>
  form.acceptedPresets([
    { key: "instantly", label: text.value.foodInstantly, patch: { foodSpawnPermille: 1000 } },
    { key: "often", label: text.value.foodOften, patch: { foodSpawnPermille: 300 } },
    { key: "rarely", label: text.value.foodRarely, patch: { foodSpawnPermille: 50 } },
  ]),
);

const wallOptions = computed(() => [
  { value: SnakeWallCollisionMode.DEATH, label: text.value.wallsDeath },
  { value: SnakeWallCollisionMode.WRAP_AROUND, label: text.value.wallsWrap },
]);

const deadSnakeOptions = computed(() => [
  { value: SnakeDeadSnakeBehavior.DISAPPEAR, label: text.value.deadSnakesVanish },
  { value: SnakeDeadSnakeBehavior.STAY_ON_FIELD, label: text.value.deadSnakesStay },
]);
</script>

<template>
  <div v-if="value" class="divide-y divide-white/5">
    <ChipsField
      :label="t.common.boardSize.label"
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
    <ChipsField
      :label="text.speed"
      :options="speedPresets"
      :unmatched-label="text.customSpeed(value.tickIntervalMs)"
      :error="form.field('tick_interval_ms').error"
      :model-value="matchingPresetKey(speedPresets, value)"
      @update:model-value="form.applyPreset(speedPresets, $event)"
    />
    <SegmentedField
      :label="text.walls"
      :help="text.wallsHelp"
      :options="wallOptions"
      :model-value="value.wallCollisionMode"
      @update:model-value="form.update({ wallCollisionMode: $event })"
    />
    <MoreOptions :has-problem="form.hasProblemIn(['max_food_count', 'food_spawn_permille'])">
      <SegmentedField
        :label="text.deadSnakes"
        :help="text.deadSnakesHelp"
        :options="deadSnakeOptions"
        :model-value="value.deadSnakeBehavior"
        @update:model-value="form.update({ deadSnakeBehavior: $event })"
      />
      <StepperField
        :label="text.foodOnField"
        :help="text.foodOnFieldHelp"
        v-bind="form.field('max_food_count')"
        :model-value="value.maxFoodCount"
        @update:model-value="form.update({ maxFoodCount: $event })"
      />
      <ChipsField
        :label="text.foodAppears"
        :help="text.foodAppearsHelp"
        :options="foodPresets"
        :unmatched-label="text.customFoodChance(value.foodSpawnPermille / 10)"
        :error="form.field('food_spawn_permille').error"
        :model-value="matchingPresetKey(foodPresets, value)"
        @update:model-value="form.applyPreset(foodPresets, $event)"
      />
    </MoreOptions>
  </div>
</template>
