<script setup lang="ts">
import { computed } from "vue";

import { t } from "@/i18n";
import { Presence } from "@/proto/minigames/v2/social_pb";

const props = defineProps<{ presence: Presence }>();

const look = computed(() => {
  switch (props.presence) {
    case Presence.PLAYING:
      return { dot: "bg-amber-400", label: t.friends.presence.playing };
    case Presence.ONLINE:
      return { dot: "bg-emerald-400", label: t.friends.presence.online };
    default:
      return { dot: "bg-gray-500", label: t.friends.presence.offline };
  }
});
</script>

<template>
  <span class="flex items-center gap-1 text-xs text-gray-400">
    <span class="size-2 shrink-0 rounded-full" :class="look.dot" aria-hidden="true" />
    {{ look.label }}
  </span>
</template>
