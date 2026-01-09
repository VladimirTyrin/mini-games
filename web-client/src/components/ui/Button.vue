<script setup lang="ts">
import { computed } from 'vue';
import Spinner from './Spinner.vue';

type ButtonVariant = 'primary' | 'secondary' | 'danger';
type ButtonSize = 'sm' | 'md' | 'lg';

interface Props {
  variant?: ButtonVariant;
  size?: ButtonSize;
  disabled?: boolean;
  loading?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'primary',
  size: 'md',
  disabled: false,
  loading: false,
});

const isDisabled = computed(() => props.disabled || props.loading);

const variantClasses = computed(() => {
  const variants: Record<ButtonVariant, string> = {
    primary: 'bg-blue-600 hover:bg-blue-700 focus:ring-blue-500 text-white',
    secondary: 'bg-slate-700 hover:bg-slate-600 focus:ring-slate-500 text-slate-300',
    danger: 'bg-red-600 hover:bg-red-700 focus:ring-red-500 text-white',
  };
  return variants[props.variant];
});

const sizeClasses = computed(() => {
  const sizes: Record<ButtonSize, string> = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-base',
    lg: 'px-6 py-3 text-lg',
  };
  return sizes[props.size];
});

const buttonClasses = computed(() => [
  'inline-flex items-center justify-center gap-2',
  'rounded-lg font-medium',
  'transition-colors duration-200',
  'focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-slate-900',
  variantClasses.value,
  sizeClasses.value,
  isDisabled.value ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer',
]);
</script>

<template>
  <button
    :class="buttonClasses"
    :disabled="isDisabled"
    type="button"
  >
    <Spinner v-if="loading" size="sm" />
    <slot />
  </button>
</template>
