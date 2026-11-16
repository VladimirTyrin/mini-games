<script setup lang="ts" generic="T extends string | number">
const model = defineModel<T>({ required: true });
defineProps<{ label: string; options: readonly { value: T; label: string }[] }>();
</script>

<template>
  <label class="flex items-center justify-between gap-3 text-sm">
    <span class="text-gray-300">{{ label }}</span>
    <select v-model="model" class="rounded bg-panel-raised px-2 py-1">
      <option v-for="option in options" :key="option.value" :value="option.value">{{ option.label }}</option>
    </select>
  </label>
</template>
