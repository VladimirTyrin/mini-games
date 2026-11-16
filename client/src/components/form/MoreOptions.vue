<script setup lang="ts">
import { computed, ref } from "vue";

const props = withDefaults(defineProps<{ label?: string; hasProblem?: boolean }>(), {
  label: "More options",
});

const expanded = ref(false);
// A rejected setting hidden in here must stay visible, or the disabled Play button is a riddle.
const isOpen = computed(() => expanded.value || props.hasProblem);
</script>

<template>
  <div>
    <button
      type="button"
      class="flex min-h-12 w-full items-center justify-between py-2 text-left text-[15px] text-gray-300 hover:text-white"
      :aria-expanded="isOpen"
      @click="expanded = !expanded"
    >
      <span>{{ label }}</span>
      <span class="text-gray-500 transition-transform" :class="isOpen ? 'rotate-180' : ''" aria-hidden="true">
        ▾
      </span>
    </button>
    <div v-if="isOpen" class="divide-y divide-white/5 border-t border-white/5">
      <slot />
    </div>
  </div>
</template>
