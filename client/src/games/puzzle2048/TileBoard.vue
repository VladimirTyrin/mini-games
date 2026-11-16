<script setup lang="ts">
import { computed } from "vue";

import type { Puzzle2048View } from "@/proto/minigames/v2/puzzle2048_pb";

import { type BoardPosition, type TileSprite, tileSprites } from "./tile-sprites";
import { tileColors, tileFontScale } from "./tile-style";

const props = defineProps<{ view: Puzzle2048View }>();

const sprites = computed(() => tileSprites(props.view));
const cellPositions = computed(() => {
  const positions: BoardPosition[] = [];
  for (let row = 0; row < props.view.height; row++)
    for (let col = 0; col < props.view.width; col++) positions.push({ row, col });
  return positions;
});

const TILE_GAP_FRACTION = 0.12;

const boardStyle = computed(() => ({ aspectRatio: `${props.view.width} / ${props.view.height}` }));

// Cells are square, so the horizontal pitch (the reference for percentage padding) fits both axes.
function slotStyle(position: BoardPosition, from: BoardPosition = position): Record<string, string> {
  const pitchPercent = 100 / props.view.width;
  return {
    width: `${pitchPercent}%`,
    height: `${100 / props.view.height}%`,
    padding: `${(pitchPercent * TILE_GAP_FRACTION) / 2}%`,
    "--row": String(position.row),
    "--col": String(position.col),
    "--from-row": String(from.row),
    "--from-col": String(from.col),
  };
}

function spriteSlotStyle(sprite: TileSprite): Record<string, string> {
  const entrance = sprite.entrance;
  const from = entrance.kind === "slide" || entrance.kind === "mergeSource" ? entrance.from : sprite.position;
  return slotStyle(sprite.position, from);
}

function tileStyle(value: number): Record<string, string> {
  const colors = tileColors(value);
  return {
    background: colors.background,
    color: colors.text,
    fontSize: `calc(100cqw / ${props.view.width} * ${tileFontScale(value)})`,
  };
}
</script>

<template>
  <div class="board" :style="boardStyle">
    <div
      v-for="position in cellPositions"
      :key="`${position.row}:${position.col}`"
      class="slot"
      :style="slotStyle(position)"
    >
      <div class="cell" />
    </div>
    <!-- Re-keyed per move so every sprite is a fresh element: animations restart from the new
         view and a frame arriving mid-animation simply replaces the old one. -->
    <div :key="view.moves" class="layer">
      <div
        v-for="sprite in sprites"
        :key="sprite.id"
        class="slot"
        :class="`entrance-${sprite.entrance.kind}`"
        :style="spriteSlotStyle(sprite)"
      >
        <div class="cell tile" :style="tileStyle(sprite.value)">{{ sprite.value }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.board {
  --slide-duration: 110ms;
  --pop-duration: 140ms;
  --spawn-delay: 150ms;
  position: relative;
  container-type: inline-size;
  border-radius: 8px;
  background: #574e44;
  box-shadow: 0 4px 20px rgb(0 0 0 / 0.3);
}

.layer {
  position: absolute;
  inset: 0;
}

.slot {
  position: absolute;
  left: 0;
  top: 0;
  transform: translate(calc(var(--col) * 100%), calc(var(--row) * 100%));
}

.cell {
  width: 100%;
  height: 100%;
  border-radius: 6%;
  background: #49423a;
}

.tile {
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  line-height: 1;
  user-select: none;
}

.entrance-slide,
.entrance-mergeSource {
  animation: tile-slide var(--slide-duration) ease-in-out both;
}

.entrance-mergeSource .tile {
  animation: tile-vanish 0ms linear var(--slide-duration) forwards;
}

.entrance-merge {
  z-index: 1;
}

.entrance-merge .tile {
  animation: tile-pop var(--pop-duration) ease-out var(--slide-duration) backwards;
}

.entrance-spawn .tile {
  animation: tile-appear var(--pop-duration) ease-out var(--spawn-delay) backwards;
}

@keyframes tile-slide {
  from {
    transform: translate(calc(var(--from-col) * 100%), calc(var(--from-row) * 100%));
  }
}

@keyframes tile-vanish {
  to {
    visibility: hidden;
  }
}

@keyframes tile-pop {
  0% {
    visibility: hidden;
    transform: scale(1);
  }
  1% {
    visibility: visible;
  }
  45% {
    transform: scale(1.18);
  }
  100% {
    transform: scale(1);
  }
}

@keyframes tile-appear {
  from {
    opacity: 0;
    transform: scale(0);
  }
}

@media (prefers-reduced-motion: reduce) {
  .slot,
  .tile {
    animation: none !important;
  }

  .entrance-mergeSource {
    display: none;
  }
}
</style>
