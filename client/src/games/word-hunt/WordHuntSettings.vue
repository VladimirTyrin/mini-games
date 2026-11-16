<script setup lang="ts">
import { useGameSettingsForm } from "@/components/form/settings-form";
import SegmentedField from "@/components/form/SegmentedField.vue";
import StepperField from "@/components/form/StepperField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { WordHuntLanguage, WordHuntPuzzleMode } from "@/proto/minigames/v2/word_hunt_pb";

import { localDayNumber } from "./daily";

const DAILY_WORD_LENGTH = 5;

const settings = defineModel<GameSettings>({ required: true });
const form = useGameSettingsForm(settings, "wordHunt");
const value = form.value;

const languageOptions = [
  { value: WordHuntLanguage.RUSSIAN, label: "Русский" },
  { value: WordHuntLanguage.ENGLISH, label: "English" },
] as const;
const modeOptions = [
  { value: WordHuntPuzzleMode.DAILY, label: "Word of the day" },
  { value: WordHuntPuzzleMode.UNLIMITED, label: "Unlimited" },
] as const;
const hardModeOptions = [
  { value: 0, label: "Off" },
  { value: 1, label: "On" },
] as const;

function setMode(mode: WordHuntPuzzleMode): void {
  // The word of the day has a fixed length (the core refuses others) and today's date.
  form.update(
    mode === WordHuntPuzzleMode.DAILY
      ? { mode, wordLength: DAILY_WORD_LENGTH, day: localDayNumber() }
      : { mode, day: 0 },
  );
}
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
      label="Guesses"
      v-bind="form.field('max_guesses')"
      :model-value="value.maxGuesses"
      @update:model-value="form.update({ maxGuesses: $event })"
    />
    <SegmentedField
      label="Hard mode"
      help="Every guess must use the letters you have found"
      :options="hardModeOptions"
      :model-value="value.hardMode ? 1 : 0"
      @update:model-value="form.update({ hardMode: $event === 1 })"
    />
    <p class="pt-3 text-xs text-gray-500">
      Word lists from OpenCorpora, SCOWL (Kevin Atkinson) and wordfreq; licensed CC BY-SA 4.0.
    </p>
  </div>
</template>
