<script setup lang="ts">
import { computed, provide, watch } from "vue";

import {
  type SettingsAudience,
  type SettingsFormState,
  settingsFormKey,
} from "@/components/form/settings-form";
import type { GamePlugin } from "@/games/game-plugin";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { type SettingsProblem, settingRanges, settingsProblem } from "@/wasm/core";

const settings = defineModel<GameSettings>({ required: true });
const props = defineProps<{ plugin: GamePlugin; seats: number; audience: SettingsAudience }>();
const emit = defineEmits<{ validity: [problem: SettingsProblem | null] }>();

// Validity and limits come from the game's own rules in the WASM core, so they live in one place.
const problem = computed(() => settingsProblem(settings.value, props.seats));
const ranges = computed(() => settingRanges(settings.value));
watch(problem, (value) => emit("validity", value), { immediate: true });

provide(
  settingsFormKey,
  computed<SettingsFormState>(() => ({
    problem: problem.value,
    ranges: ranges.value,
    seats: props.seats,
    audience: props.audience,
  })),
);
</script>

<template>
  <div>
    <component :is="plugin.SettingsForm" v-model="settings" />
    <!-- Problems not tied to one row; the line is always there so nothing below it moves. -->
    <p class="min-h-5 pt-1 text-xs text-rose-300" aria-live="polite">
      {{ problem && problem.field === null ? problem.message : "" }}
    </p>
  </div>
</template>
