<script setup lang="ts">
import { computed } from "vue";

import { t } from "@/i18n";

import SettingRow from "./SettingRow.vue";

const model = defineModel<number>({ required: true });
const props = defineProps<{
  label: string;
  help?: string;
  /** Limits come from the core (`useGameSettingsForm().field`); absent means unbounded. */
  min?: number;
  max?: number;
  error?: string | null;
}>();

const canDecrease = computed(() => props.min === undefined || model.value > props.min);
const canIncrease = computed(() => props.max === undefined || model.value < props.max);

// A value outside the limits (old remembered settings) steps straight back inside them.
function clamped(value: number): number {
  const atLeastMin = props.min === undefined ? value : Math.max(props.min, value);
  return props.max === undefined ? atLeastMin : Math.min(props.max, atLeastMin);
}

function stepBy(direction: 1 | -1): void {
  model.value = clamped(model.value + direction);
}
</script>

<template>
  <SettingRow :label="label" :help="help" :error="error">
    <div class="flex w-full items-center justify-between rounded-lg bg-panel-raised">
      <button
        type="button"
        class="flex size-11 items-center justify-center rounded-lg text-xl text-gray-200 hover:bg-white/5"
        :aria-label="t.match.form.decrease(label)"
        :disabled="!canDecrease"
        @click="stepBy(-1)"
      >
        −
      </button>
      <output
        class="min-w-0 flex-1 text-center text-base font-semibold tabular-nums"
        :class="error ? 'text-rose-300' : 'text-gray-100'"
        aria-live="polite"
      >
        {{ model }}
      </output>
      <button
        type="button"
        class="flex size-11 items-center justify-center rounded-lg text-xl text-gray-200 hover:bg-white/5"
        :aria-label="t.match.form.increase(label)"
        :disabled="!canIncrease"
        @click="stepBy(1)"
      >
        +
      </button>
    </div>
  </SettingRow>
</template>
