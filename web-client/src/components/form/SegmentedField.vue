<script setup lang="ts" generic="T extends string | number">
import SettingRow from "./SettingRow.vue";

const model = defineModel<T>({ required: true });
defineProps<{
  label: string;
  help?: string;
  options: readonly { value: T; label: string }[];
  /** Three options need the full row width on a phone. */
  stacked?: boolean;
}>();
</script>

<template>
  <SettingRow :label="label" :help="help" :stacked="stacked">
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
