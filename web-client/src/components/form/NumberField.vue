<script setup lang="ts">
const model = defineModel<number>({ required: true });
defineProps<{ label: string; step?: number; suffix?: string }>();

// Keeps the model numeric: an empty or partial input never reaches the settings message.
function onInput(event: Event): void {
  const value = (event.target as HTMLInputElement).valueAsNumber;
  if (Number.isFinite(value)) model.value = value;
}
</script>

<template>
  <label class="flex items-center justify-between gap-3 text-sm">
    <span class="text-gray-300">{{ label }}</span>
    <span class="flex items-center gap-1">
      <input
        type="number"
        inputmode="numeric"
        class="w-24 rounded bg-panel-raised px-2 py-1 text-right"
        :step="step ?? 1"
        :value="model"
        @input="onInput"
      />
      <span v-if="suffix" class="text-gray-400">{{ suffix }}</span>
    </span>
  </label>
</template>
