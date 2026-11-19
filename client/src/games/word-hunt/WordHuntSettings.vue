<script setup lang="ts">
import { computed, onMounted, watch } from "vue";

import { useGameSettingsForm } from "@/components/form/settings-form";
import SegmentedField from "@/components/form/SegmentedField.vue";
import StepperField from "@/components/form/StepperField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { WordHuntFirstTurn, WordHuntLanguage, WordHuntPuzzleMode } from "@/proto/minigames/v2/word_hunt_pb";
import { t } from "@/i18n";

import { localDayNumber } from "./daily";
import { marksMissingLetters } from "./preferences";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "wordHunt");
const value = form.value;

const texts = computed(() => t.games.wordHunt.settings);
const languageOptions = computed(() => [
  { value: WordHuntLanguage.RUSSIAN, label: t.games.wordHunt.wordLanguages.russian },
  { value: WordHuntLanguage.ENGLISH, label: t.games.wordHunt.wordLanguages.english },
]);
/** With opponents: turns on one word; the word of the day is for playing alone. */
const isGroup = computed(() => form.seats.value > 1);
const modeOptions = computed(() => {
  const unlimited = { value: WordHuntPuzzleMode.UNLIMITED, label: texts.value.unlimited };
  if (isGroup.value) return [unlimited];
  return [{ value: WordHuntPuzzleMode.DAILY, label: texts.value.wordOfTheDay }, unlimited];
});
const firstTurnOptions = computed(() => [
  { value: WordHuntFirstTurn.RANDOM, label: t.common.random },
  { value: WordHuntFirstTurn.HOST_FIRST, label: texts.value.firstSeatFirst[form.audience.value] },
  { value: WordHuntFirstTurn.HOST_LAST, label: texts.value.firstSeatLast[form.audience.value] },
]);
const offOnOptions = computed(() => [
  { value: 0, label: texts.value.off },
  { value: 1, label: texts.value.on },
]);

function setLanguage(language: WordHuntLanguage): void {
  form.update({ language });
}

function setMarksMissing(marks: boolean): void {
  marksMissingLetters.value = marks;
}

function setMode(mode: WordHuntPuzzleMode): void {
  if (mode !== WordHuntPuzzleMode.DAILY) {
    form.update({ mode, day: 0 });
    return;
  }
  form.update({ mode, day: localDayNumber() });
}

// The word of the day has the one length the core publishes for it; the ranges follow the mode
// once the core has seen it, so the length is fitted afterwards.
watch(
  () => [value.value?.mode, form.field("word_length").min, form.field("word_length").max] as const,
  ([mode, min, max]) => {
    const length = value.value?.wordLength;
    if (mode !== WordHuntPuzzleMode.DAILY || min === undefined || min !== max || length === min) return;
    form.update({ wordLength: min });
  },
  { immediate: true },
);

// Adding an opponent leaves the word of the day behind.
watch(
  isGroup,
  (group) => {
    if (group && value.value?.mode === WordHuntPuzzleMode.DAILY) setMode(WordHuntPuzzleMode.UNLIMITED);
  },
  { immediate: true },
);

// Remembered settings (and a lobby's) may carry an earlier day.
onMounted(() => {
  if (value.value?.mode === WordHuntPuzzleMode.DAILY) setMode(WordHuntPuzzleMode.DAILY);
});
</script>

<template>
  <div v-if="value" class="divide-y divide-white/5">
    <SegmentedField
      :label="texts.language"
      :options="languageOptions"
      :model-value="value.language"
      @update:model-value="setLanguage"
    />
    <SegmentedField
      v-if="!isGroup"
      :label="texts.word"
      :help="texts.wordHelp"
      stacked
      :options="modeOptions"
      :model-value="value.mode"
      @update:model-value="setMode"
    />
    <StepperField
      v-if="value.mode !== WordHuntPuzzleMode.DAILY"
      :label="texts.letters"
      v-bind="form.field('word_length')"
      :model-value="value.wordLength"
      @update:model-value="form.update({ wordLength: $event })"
    />
    <StepperField
      v-if="!isGroup"
      :label="texts.guesses"
      v-bind="form.field('max_guesses')"
      :model-value="value.maxGuesses"
      @update:model-value="form.update({ maxGuesses: $event })"
    />
    <SegmentedField
      v-if="isGroup"
      :label="texts.firstTurn"
      :help="texts.firstTurnHelp"
      stacked
      :options="firstTurnOptions"
      :model-value="
        value.firstTurn === WordHuntFirstTurn.UNSPECIFIED ? WordHuntFirstTurn.RANDOM : value.firstTurn
      "
      @update:model-value="form.update({ firstTurn: $event })"
    />
    <SegmentedField
      :label="texts.hardMode"
      :help="texts.hardModeHelp"
      :options="offOnOptions"
      :model-value="value.hardMode ? 1 : 0"
      @update:model-value="form.update({ hardMode: $event === 1 })"
    />
    <SegmentedField
      :label="texts.markMissing"
      :help="texts.markMissingHelp"
      :options="offOnOptions"
      :model-value="marksMissingLetters ? 1 : 0"
      @update:model-value="setMarksMissing($event === 1)"
    />
    <p class="pt-3 text-xs text-gray-500">{{ texts.credits }}</p>
  </div>
</template>
