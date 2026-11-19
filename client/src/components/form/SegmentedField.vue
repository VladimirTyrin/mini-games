<script setup lang="ts" generic="T extends string | number">
import { computed } from "vue";

import SettingRow from "./SettingRow.vue";

const model = defineModel<T>({ required: true });
const props = defineProps<{
  label: string;
  help?: string;
  options: readonly { value: T; label: string }[];
  /** Three options need the full row width on a phone. */
  stacked?: boolean;
}>();
/**
 * The inline column (SettingRow's w-38) holds about 13 letters across its options; longer
 * labels (Russian's, often) take the row's full width below the label instead of being cut.
 */
const fitsInline = computed(
  () =>
    Math.max(...props.options.map((option) => option.label.length)) <= Math.floor(13 / props.options.length),
);
</script>

<template>
  <SettingRow :label="label" :help="help" :stacked="stacked || !fitsInline">
    <div role="radiogroup" :aria-label="label" class="flex w-full gap-0.5 rounded-lg bg-panel-raised p-0.5">
      <button
        v-for="option in options"
        :key="option.value"
        type="button"
        role="radio"
        :aria-checked="option.value === model"
        class="h-11 min-w-0 flex-1 truncate rounded-md px-2 text-sm font-medium transition-colors"
        :class="
          option.value === model ? 'bg-emerald-600 text-white shadow' : 'text-gray-300 hover:bg-white/5'
        "
        @click="model = option.value"
      >
        {{ option.label }}
      </button>
    </div>
  </SettingRow>
</template>
