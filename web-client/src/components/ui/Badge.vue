<script setup lang="ts">
import { computed } from 'vue';

type BadgeVariant = 'success' | 'warning' | 'error' | 'info' | 'default';

interface Props {
  variant?: BadgeVariant;
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'default',
});

const variantClasses = computed(() => {
  const variants: Record<BadgeVariant, string> = {
    success: 'bg-green-600/20 text-green-400 border-green-600/30',
    warning: 'bg-yellow-600/20 text-yellow-400 border-yellow-600/30',
    error: 'bg-red-600/20 text-red-400 border-red-600/30',
    info: 'bg-blue-600/20 text-blue-400 border-blue-600/30',
    default: 'bg-slate-600/20 text-slate-300 border-slate-600/30',
  };
  return variants[props.variant];
});

const badgeClasses = computed(() => [
  'inline-flex items-center',
  'px-2.5 py-0.5',
  'text-xs font-medium',
  'rounded-full border',
  variantClasses.value,
]);
</script>

<template>
  <span :class="badgeClasses">
    <slot />
  </span>
</template>
