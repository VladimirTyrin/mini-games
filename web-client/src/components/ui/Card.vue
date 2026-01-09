<script setup lang="ts">
interface Props {
  title?: string;
}

withDefaults(defineProps<Props>(), {
  title: '',
});
</script>

<template>
  <div class="bg-slate-800 border border-slate-700 rounded-lg overflow-hidden">
    <div v-if="title" class="px-6 py-4 border-b border-slate-700">
      <h3 class="text-lg font-semibold text-white">
        {{ title }}
      </h3>
    </div>
    <div class="p-6">
      <slot />
    </div>
  </div>
</template>
