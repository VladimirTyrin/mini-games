<script setup lang="ts">
import { computed } from "vue";

import ChipsField from "@/components/form/ChipsField.vue";
import { matchingPresetKey } from "@/components/form/presets";
import SegmentedField from "@/components/form/SegmentedField.vue";
import { TWO_SEAT_NAMES, useGameSettingsForm } from "@/components/form/settings-form";
import StepperField from "@/components/form/StepperField.vue";
import { t } from "@/i18n";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { TicTacToeFirstPlayerMode } from "@/proto/minigames/v2/tictactoe_pb";

import { boardPresets } from "./board-presets";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "tictactoe");
const { value, audience } = form;
const text = computed(() => t.games.tictactoe.settings);

const sizePresets = computed(() => form.acceptedPresets(boardPresets()));

const firstPlayerOptions = computed(() => {
  const [first, second] = TWO_SEAT_NAMES[audience.value];
  return [
    { value: TicTacToeFirstPlayerMode.HOST, label: first },
    { value: TicTacToeFirstPlayerMode.GUEST, label: second },
    { value: TicTacToeFirstPlayerMode.RANDOM, label: t.common.random },
  ];
});
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
        <StepperField
          :label="text.inARowToWin"
          v-bind="form.field('win_count')"
          :model-value="value.winCount"
          @update:model-value="form.update({ winCount: $event })"
        />
      </template>
    </ChipsField>
    <SegmentedField
      :label="text.firstMove"
      :help="text.firstMoveHelp"
      stacked
      :options="firstPlayerOptions"
      :model-value="value.firstPlayer"
      @update:model-value="form.update({ firstPlayer: $event })"
    />
  </div>
</template>
