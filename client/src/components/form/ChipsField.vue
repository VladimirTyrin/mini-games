<script setup lang="ts">
import { computed, ref } from "vue";

import SettingRow from "./SettingRow.vue";

export interface ChipOption {
  key: string;
  label: string;
  detail?: string;
}

/** Key of the preset the current settings match; `null` when they match none. */
const model = defineModel<string | null>({ required: true });
const props = defineProps<{
  label: string;
  help?: string;
  options: readonly ChipOption[];
  /** Adds a chip that reveals the `custom` slot (fine-grained controls). */
  customLabel?: string;
  /** Without a custom chip: a passive chip naming the current value when no preset matches. */
  unmatchedLabel?: string;
  error?: string | null;
}>();

// Picking "Custom" keeps the slot open even while the values still match a preset.
const customRequested = ref(false);
const showsCustom = computed(() => !!props.customLabel && (customRequested.value || model.value === null));
const showsUnmatched = computed(() => !props.customLabel && !!props.unmatchedLabel && model.value === null);

function pick(key: string): void {
  customRequested.value = false;
  model.value = key;
}

function chipClass(selected: boolean): string {
  return selected
    ? "border-emerald-400 bg-emerald-600/25 text-white"
    : "border-white/10 bg-panel-raised text-gray-200 hover:bg-white/5";
}
</script>

<template>
  <SettingRow :label="label" :help="help" :error="error" stacked>
    <div class="w-full">
      <div role="radiogroup" :aria-label="label" class="grid grid-flow-col auto-cols-fr gap-1.5">
        <button
          v-for="option in options"
          :key="option.key"
          type="button"
          role="radio"
          :aria-checked="!showsCustom && model === option.key"
          :aria-label="option.detail ? `${option.label}, ${option.detail}` : option.label"
          class="flex min-h-12 min-w-0 flex-col items-center justify-center rounded-lg border px-1 py-1 leading-tight"
          :class="chipClass(!showsCustom && model === option.key)"
          @click="pick(option.key)"
        >
          <!-- A notch smaller below 360 px, so four chips keep their labels on a 320 px phone. -->
          <span class="max-w-full truncate text-sm font-semibold max-[359px]:text-xs">{{
            option.label
          }}</span>
          <span v-if="option.detail" class="max-w-full truncate text-[11px] text-gray-400">
            {{ option.detail }}
          </span>
        </button>
        <button
          v-if="customLabel"
          type="button"
          role="radio"
          :aria-checked="showsCustom"
          class="flex min-h-12 min-w-0 flex-col items-center justify-center rounded-lg border px-1 py-1 text-sm font-semibold"
          :class="chipClass(showsCustom)"
          @click="customRequested = true"
        >
          {{ customLabel }}
        </button>
        <span
          v-if="showsUnmatched"
          role="radio"
          aria-checked="true"
          aria-disabled="true"
          class="flex min-h-12 min-w-0 items-center justify-center rounded-lg border px-1 text-center text-xs font-semibold"
          :class="chipClass(true)"
        >
          {{ unmatchedLabel }}
        </span>
      </div>
      <div v-if="showsCustom" class="mt-1 divide-y divide-white/5 border-l-2 border-emerald-600/40 pl-3">
        <slot name="custom" />
      </div>
    </div>
  </SettingRow>
</template>
