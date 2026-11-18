<script setup lang="ts">
import { computed } from "vue";

import SegmentedField from "@/components/form/SegmentedField.vue";
import { TWO_SEAT_NAMES, useGameSettingsForm } from "@/components/form/settings-form";
import { CheckersCaptureRule, CheckersWhitePlayer } from "@/proto/minigames/v2/checkers_pb";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "checkers");
const { value, audience } = form;

const whiteOptions = computed(() => {
  const [first, second] = TWO_SEAT_NAMES[audience.value];
  return [
    { value: CheckersWhitePlayer.HOST, label: first },
    { value: CheckersWhitePlayer.GUEST, label: second },
    { value: CheckersWhitePlayer.RANDOM, label: "Random" },
  ];
});

const captureOptions = [
  { value: CheckersCaptureRule.TO_THE_END, label: "To the end" },
  { value: CheckersCaptureRule.MAY_STOP, label: "May stop" },
];
</script>

<template>
  <div v-if="value" class="divide-y divide-white/5">
    <SegmentedField
      label="Plays white"
      help="White moves first"
      stacked
      :options="whiteOptions"
      :model-value="value.whitePlayer"
      @update:model-value="form.update({ whitePlayer: $event })"
    />
    <SegmentedField
      label="Captures"
      help="Russian rules take every piece a capture can reach"
      stacked
      :options="captureOptions"
      :model-value="
        value.captureRule === CheckersCaptureRule.MAY_STOP
          ? CheckersCaptureRule.MAY_STOP
          : CheckersCaptureRule.TO_THE_END
      "
      @update:model-value="form.update({ captureRule: $event })"
    />
  </div>
</template>
