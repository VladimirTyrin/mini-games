<script setup lang="ts">
import { computed } from 'vue';

interface Props {
  modelValue?: string;
  type?: string;
  placeholder?: string;
  disabled?: boolean;
  error?: string;
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
  type: 'text',
  placeholder: '',
  disabled: false,
  error: '',
});

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const inputClasses = computed(() => [
  'w-full px-4 py-2',
  'bg-slate-800 text-white placeholder-slate-400',
  'border rounded-lg',
  'transition-colors duration-200',
  'focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-slate-900',
  props.error
    ? 'border-red-500 focus:ring-red-500'
    : 'border-slate-600 focus:border-blue-500 focus:ring-blue-500',
  props.disabled ? 'opacity-50 cursor-not-allowed' : '',
]);

function onInput(event: Event) {
  const target = event.target as HTMLInputElement;
  emit('update:modelValue', target.value);
}
</script>

<template>
  <div class="w-full">
    <label v-if="$slots.label" class="block mb-2 text-sm font-medium text-slate-300">
      <slot name="label" />
    </label>
    <input
      :type="type"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      :class="inputClasses"
      @input="onInput"
    />
    <p v-if="error" class="mt-1 text-sm text-red-500">
      {{ error }}
    </p>
  </div>
</template>
