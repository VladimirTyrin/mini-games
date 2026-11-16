<script setup lang="ts">
import { useAppUpdateStore } from "@/stores/app-update";

const update = useAppUpdateStore();
</script>

<template>
  <div v-if="update.available" class="flex items-center gap-3 rounded-xl bg-emerald-900/60 p-3 text-sm">
    <span class="flex-1">
      Update available: <span class="font-semibold">{{ update.available.versionName }}</span>
    </span>
    <button class="rounded-full bg-emerald-600 px-4 py-2 font-semibold" @click="update.install()">
      Download
    </button>
  </div>
</template>
