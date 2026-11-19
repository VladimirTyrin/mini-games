<script setup lang="ts">
import { computed, ref, watch } from "vue";

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

/**
 * What is typed in the field. Applied on Enter or on leaving the field, not on every key: typing
 * 15 goes through 1, which the limits would turn into the minimum at once.
 */
const draft = ref(String(model.value));
watch(model, (value) => (draft.value = String(value)));

function applyDraft(): void {
  const typed = Number.parseInt(draft.value.trim(), 10);
  // Not a number: the field shows the value it had.
  if (Number.isNaN(typed)) draft.value = String(model.value);
  else {
    model.value = clamped(typed);
    draft.value = String(model.value);
  }
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
      <input
        v-model="draft"
        type="text"
        inputmode="numeric"
        enterkeyhint="done"
        autocomplete="off"
        :aria-label="label"
        class="w-0 min-w-0 flex-1 bg-transparent text-center text-base font-semibold tabular-nums outline-none focus:rounded focus:ring-1 focus:ring-emerald-500/60"
        :class="error ? 'text-rose-300' : 'text-gray-100'"
        @change="applyDraft"
        @keydown.enter.prevent="($event.target as HTMLInputElement).blur()"
        @keydown.up.prevent="stepBy(1)"
        @keydown.down.prevent="stepBy(-1)"
      />
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
