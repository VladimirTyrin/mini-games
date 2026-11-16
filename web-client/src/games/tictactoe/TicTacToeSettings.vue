<script setup lang="ts">
import { computed } from "vue";

import NumberField from "@/components/form/NumberField.vue";
import SelectField from "@/components/form/SelectField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { type TicTacToeSettings, TicTacToeFirstPlayerMode } from "@/proto/minigames/v2/tictactoe_pb";

const settings = defineModel<GameSettings>({ required: true });

const value = computed(() => (settings.value.game.case === "tictactoe" ? settings.value.game.value : null));

function update(patch: Partial<Omit<TicTacToeSettings, "$typeName">>): void {
  if (!value.value) return;
  settings.value = { ...settings.value, game: { case: "tictactoe", value: { ...value.value, ...patch } } };
}

const firstPlayerOptions = [
  { value: TicTacToeFirstPlayerMode.RANDOM, label: "Random" },
  { value: TicTacToeFirstPlayerMode.HOST, label: "Host" },
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
    <NumberField
      label="In a row to win"
      :model-value="value.winCount"
      @update:model-value="update({ winCount: $event })"
    />
    <SelectField
      label="First move"
      :options="firstPlayerOptions"
      :model-value="value.firstPlayer"
      @update:model-value="update({ firstPlayer: $event })"
    />
  </div>
</template>
