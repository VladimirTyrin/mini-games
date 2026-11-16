<script setup lang="ts">
import { computed, watch } from "vue";

import type { GamePlugin } from "@/games/game-plugin";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { settingsProblem } from "@/wasm/core";

const settings = defineModel<GameSettings>({ required: true });
const props = defineProps<{ plugin: GamePlugin; seats: number }>();
const emit = defineEmits<{ validity: [problem: string | null] }>();

// Validity is decided by the game's own rules in the WASM core, so limits live in one place.
const problem = computed(() => settingsProblem(settings.value, props.seats));
watch(problem, (value) => emit("validity", value), { immediate: true });
</script>

<template>
  <div class="space-y-3">
    <component :is="plugin.SettingsForm" v-model="settings" />
    <p v-if="problem" class="rounded bg-rose-950/60 px-3 py-2 text-sm text-rose-200">{{ problem }}</p>
  </div>
</template>
