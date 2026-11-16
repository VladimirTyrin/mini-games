<script setup lang="ts">
import { computed, ref, watch } from "vue";

/** `playerId` picks the fallback colour, so namesakes without pictures still look different. */
const props = defineProps<{ url: string; name: string; playerId?: string }>();

const FALLBACK_TONES = ["bg-emerald-600", "bg-sky-600", "bg-violet-600", "bg-amber-600", "bg-rose-600"];

const failed = ref(false);
watch(
  () => props.url,
  () => (failed.value = false),
);

const showsImage = computed(() => props.url !== "" && !failed.value);
const initial = computed(() => Array.from(props.name.trim())[0]?.toUpperCase() ?? "?");
const tone = computed(() => {
  let sum = 0;
  for (const character of props.playerId ?? props.name) sum += character.codePointAt(0) ?? 0;
  return FALLBACK_TONES[sum % FALLBACK_TONES.length];
});
</script>

<template>
  <img
    v-if="showsImage"
    :src="url"
    alt=""
    referrerpolicy="no-referrer"
    loading="lazy"
    class="inline-block h-6 w-6 shrink-0 rounded-full object-cover"
    @error="failed = true"
  />
  <span
    v-else
    aria-hidden="true"
    class="inline-flex h-6 w-6 shrink-0 select-none items-center justify-center rounded-full text-xs font-semibold text-white"
    :class="tone"
    >{{ initial }}</span
  >
</template>
