<script setup lang="ts">
import { computed } from 'vue';

type SpinnerSize = 'sm' | 'md' | 'lg';

interface Props {
  size?: SpinnerSize;
}

const props = withDefaults(defineProps<Props>(), {
  size: 'md',
});

const sizeClasses = computed(() => {
  const sizes: Record<SpinnerSize, string> = {
    sm: 'w-4 h-4 border-2',
    md: 'w-6 h-6 border-2',
    lg: 'w-10 h-10 border-3',
  };
  return sizes[props.size];
});

const spinnerClasses = computed(() => [
  'inline-block',
  'rounded-full',
  'border-current border-t-transparent',
  'animate-spin',
  sizeClasses.value,
]);
</script>

<template>
  <div :class="spinnerClasses" role="status" aria-label="Loading">
    <span class="sr-only">Loading...</span>
  </div>
</template>
