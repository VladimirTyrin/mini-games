<script setup lang="ts">
import type { MatchController } from "@/match/match-controller";

defineProps<{ controller: MatchController }>();
</script>

<template>
  <pre class="overflow-auto text-xs">{{ controller.current.value?.view.game.case }}</pre>
</template>
