<script setup lang="ts">
import Avatar from "@/components/Avatar.vue";
import PresenceDot from "@/components/PresenceDot.vue";
import type { Friend } from "@/proto/minigames/v2/social_pb";

/** The `status` slot takes the presence's place (an invitation's state); the default slot holds actions. */
defineProps<{ friend: Friend }>();
</script>

<template>
  <li class="flex items-center gap-2">
    <Avatar
      :url="friend.identity?.avatarUrl ?? ''"
      :name="friend.identity?.displayName ?? ''"
      :player-id="friend.identity?.id"
    />
    <span class="min-w-0 flex-1">
      <span class="block text-sm break-words">{{ friend.identity?.displayName }}</span>
      <slot name="status">
        <PresenceDot :presence="friend.presence" />
      </slot>
    </span>
    <slot />
  </li>
</template>
