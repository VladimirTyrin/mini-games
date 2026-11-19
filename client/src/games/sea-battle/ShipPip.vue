<script setup lang="ts">
import { t } from "@/i18n";

import ShipSprite from "./ShipSprite.vue";

/** A small ship for a list of ships, drawn like the ships on the sea. */
defineProps<{
  length: number;
  /** Solid: placed or afloat. Outlined: still to place. */
  look: "solid" | "outline";
  size: "small" | "medium";
}>();

/** The height of one deck, in pixels. */
const DECK_PX = { small: 12, medium: 14 } as const;
</script>

<template>
  <span
    class="inline-block"
    :style="{ width: `${length * DECK_PX[size]}px`, height: `${DECK_PX[size]}px` }"
    :aria-label="t.games.seaBattle.ship(length)"
  >
    <ShipSprite :length="length" horizontal :look="look === 'solid' ? 'afloat' : 'ghost'" />
  </span>
</template>
