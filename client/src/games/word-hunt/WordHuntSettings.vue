<script setup lang="ts">
import { computed, onMounted, watch } from "vue";

import { useGameSettingsForm } from "@/components/form/settings-form";
import SegmentedField from "@/components/form/SegmentedField.vue";
import StepperField from "@/components/form/StepperField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { WordHuntFirstTurn, WordHuntLanguage, WordHuntPuzzleMode } from "@/proto/minigames/v2/word_hunt_pb";

import { localDayNumber } from "./daily";
import { marksMissingLetters } from "./preferences";

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "wordHunt");
const value = form.value;

const languageOptions = [
  { value: WordHuntLanguage.RUSSIAN, label: "Русский" },
  { value: WordHuntLanguage.ENGLISH, label: "English" },
] as const;
const DAILY_OPTION = { value: WordHuntPuzzleMode.DAILY, label: "Word of the day" } as const;
const UNLIMITED_OPTION = { value: WordHuntPuzzleMode.UNLIMITED, label: "Unlimited" } as const;
/** With opponents: turns on one word; the word of the day is for playing alone. */
const isGroup = computed(() => form.seats.value > 1);
const modeOptions = computed(() => (isGroup.value ? [UNLIMITED_OPTION] : [DAILY_OPTION, UNLIMITED_OPTION]));
const firstTurnOptions = computed(() => {
  const local = form.audience.value === "local";
  return [
    { value: WordHuntFirstTurn.RANDOM, label: "Random" },
    { value: WordHuntFirstTurn.HOST_FIRST, label: local ? "Me first" : "Host first" },
    { value: WordHuntFirstTurn.HOST_LAST, label: local ? "Me last" : "Host last" },
  ];
});
const offOnOptions = [
  { value: 0, label: "Off" },
  { value: 1, label: "On" },
] as const;

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
      label="Language"
      :options="languageOptions"
      :model-value="value.language"
      @update:model-value="form.update({ language: $event })"
    />
    <SegmentedField
      v-if="!isGroup"
      label="Word"
      help="The word of the day is the same for everyone"
      stacked
      :options="modeOptions"
      :model-value="value.mode"
      @update:model-value="setMode"
    />
    <StepperField
      v-if="value.mode !== WordHuntPuzzleMode.DAILY"
      label="Letters"
      v-bind="form.field('word_length')"
      :model-value="value.wordLength"
      @update:model-value="form.update({ wordLength: $event })"
    />
    <StepperField
      v-if="!isGroup"
      label="Guesses"
      v-bind="form.field('max_guesses')"
      :model-value="value.maxGuesses"
      @update:model-value="form.update({ maxGuesses: $event })"
    />
    <SegmentedField
      v-if="isGroup"
      label="First turn"
      help="Turns then go round the table"
      stacked
      :options="firstTurnOptions"
      :model-value="
        value.firstTurn === WordHuntFirstTurn.UNSPECIFIED ? WordHuntFirstTurn.RANDOM : value.firstTurn
      "
      @update:model-value="form.update({ firstTurn: $event })"
    />
    <SegmentedField
      label="Hard mode"
      help="Every guess must use the letters you have found"
      :options="offOnOptions"
      :model-value="value.hardMode ? 1 : 0"
      @update:model-value="form.update({ hardMode: $event === 1 })"
    />
    <SegmentedField
      label="Mark missing letters"
      help="Darken keyboard letters that are not in the word"
      :options="offOnOptions"
      :model-value="marksMissingLetters ? 1 : 0"
      @update:model-value="setMarksMissing($event === 1)"
    />
    <p class="pt-3 text-xs text-gray-500">
      Word lists from OpenCorpora, SCOWL (Kevin Atkinson) and wordfreq; licensed CC BY-SA 4.0.
    </p>
  </div>
</template>
