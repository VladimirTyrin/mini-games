<script setup lang="ts">
import { computed } from "vue";

import { WordHuntLetterMark, type WordHuntKeyHint } from "@/proto/minigames/v2/word_hunt_pb";

import { markClass } from "./mark-style";

const props = defineProps<{
  rows: readonly string[];
  hints: readonly WordHuntKeyHint[];
  disabled: boolean;
  /** Darken letters known to be missing; otherwise they look like letters not tried yet. */
  marksMissing: boolean;
}>();
const emit = defineEmits<{ letter: [letter: string]; erase: [] }>();

const markOf = computed(() => new Map(props.hints.map((hint) => [hint.letter, hint.mark])));

const UNTRIED = "bg-gray-500/40 text-gray-100";
/** Still pressable: a missing letter may be typed on purpose, e.g. to rule others out. */
const MISSING = "bg-gray-900 text-gray-600 ring-1 ring-inset ring-white/5";

function keyClass(letter: string): string {
  const mark = markOf.value.get(letter);
  switch (mark) {
    case WordHuntLetterMark.CORRECT:
    case WordHuntLetterMark.PRESENT:
      return markClass(mark);
    case WordHuntLetterMark.ABSENT:
      return props.marksMissing ? MISSING : UNTRIED;
    default:
      return UNTRIED;
  }
}

// pointerdown: the key acts the moment the finger lands, and the button never takes focus
// (Enter on a focused key would type the letter again).
function press(event: PointerEvent, action: () => void): void {
  event.preventDefault();
  if (!props.disabled) action();
}

/** Keyboard (Tab, Enter) and screen-reader activation send a click with no pointer. */
function activate(event: MouseEvent, action: () => void): void {
  if (event.detail === 0 && !props.disabled) action();
}
</script>

<template>
  <div class="flex w-full max-w-xl select-none flex-col gap-1.5" @contextmenu.prevent>
    <div v-for="(row, index) in rows" :key="row" class="flex justify-center gap-1">
      <button
        v-for="letter in row"
        :key="letter"
        type="button"
        class="h-12 min-w-0 flex-1 rounded-md text-base font-semibold uppercase transition-colors active:brightness-125"
        :class="keyClass(letter)"
        @pointerdown="press($event, () => emit('letter', letter))"
        @click="activate($event, () => emit('letter', letter))"
      >
        {{ letter }}
      </button>
      <button
        v-if="index === rows.length - 1"
        type="button"
        aria-label="Delete letter"
        class="h-12 min-w-0 flex-[1.6] rounded-md bg-gray-500/40 text-lg text-gray-100 active:brightness-125"
        @pointerdown="press($event, () => emit('erase'))"
        @click="activate($event, () => emit('erase'))"
      >
        ⌫
      </button>
    </div>
  </div>
</template>
