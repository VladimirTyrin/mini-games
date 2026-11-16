<script setup lang="ts">
import type { MessageInitShape } from "@bufbuild/protobuf";
import { computed, onBeforeUnmount, ref, watch } from "vue";

import { useGameKeys } from "@/input/use-game-keys";
import { CommandRejectedError } from "@/match/command-rejected-error";
import type { MatchController } from "@/match/match-controller";
import {
  NumbersMatchCellState,
  type NumbersMatchCommandSchema,
  NumbersMatchHintMode,
  NumbersMatchLossReason,
  NumbersMatchPhase,
} from "@/proto/minigames/v2/numbers_match_pb";
import { useToastStore } from "@/stores/toasts";

import ActionButton from "./ActionButton.vue";
import CellGrid from "./CellGrid.vue";

const props = defineProps<{ controller: MatchController }>();
const toasts = useToastStore();
const selectedIndex = ref<number | null>(null);

/** A pair the core refused, shown briefly with the reason so the player sees what was wrong. */
interface Rejection {
  readonly cells: ReadonlySet<number>;
  readonly reason: string;
  /** Restarts the shake when the same cells are refused again. */
  readonly serial: number;
}
const REJECTION_SHOWN_MS = 2_500;
const NO_CELLS: ReadonlySet<number> = new Set();
const rejection = ref<Rejection | null>(null);
let rejectionSerial = 0;
let rejectionTimer: ReturnType<typeof setTimeout> | undefined;
onBeforeUnmount(() => clearTimeout(rejectionTimer));
const sending = ref(false);

const view = computed(() => {
  const game = props.controller.current.value?.view.game;
  return game?.case === "numbersMatch" ? game.value : null;
});
const canPlay = computed(
  () => props.controller.mySlot !== null && view.value?.phase === NumbersMatchPhase.PLAYING,
);

const hintedIndices = computed(() => {
  const hint = view.value?.hint?.hint;
  return new Set(hint?.case === "pair" ? [hint.value.firstIndex, hint.value.secondIndex] : []);
});
const suggestsRefill = computed(() => view.value?.hint?.hint.case === "suggestRefill");
const hintsEnabled = computed(() => view.value?.hintMode !== NumbersMatchHintMode.DISABLED);
const hintsLeftText = computed(() => String(view.value?.hintsLeft ?? "∞"));
const activeCellCount = computed(
  () => view.value?.cells.filter((cell) => cell.state === NumbersMatchCellState.ACTIVE).length ?? 0,
);

const status = computed(() => {
  if (rejection.value) return { text: rejection.value.reason, tone: "text-rose-300" };
  const current = view.value;
  switch (current?.phase) {
    case NumbersMatchPhase.WON:
      return { text: "Board cleared!", tone: "text-emerald-300" };
    case NumbersMatchPhase.LOST:
      return current.lossReason === NumbersMatchLossReason.DROPPED
        ? { text: "Game abandoned", tone: "text-gray-400" }
        : { text: "No moves left", tone: "text-rose-400" };
    case NumbersMatchPhase.PLAYING:
      return suggestsRefill.value
        ? { text: "No pairs left — refill!", tone: "text-amber-300" }
        : { text: `${activeCellCount.value} numbers left`, tone: "text-gray-400" };
    default:
      return { text: "Dealing…", tone: "text-gray-400" };
  }
});

// Indices refer to the board they were picked on; any change except a shown hint can shift them.
watch(view, (current) => {
  if (current?.lastAction?.action.case !== "hintShown") selectedIndex.value = null;
  rejection.value = null;
});

/** Why the core refused the pair. The value rule is simple to tell; otherwise it is distance. */
function rejectionReason(first: number, second: number): string {
  const cells = view.value?.cells ?? [];
  const a = cells[first]?.value ?? 0;
  const b = cells[second]?.value ?? 0;
  return a === b || a + b === 10
    ? "Too far apart: pairs must touch, or have only crossed-out cells between them"
    : "Not a pair: equal numbers or a sum of 10";
}

function showRejection(first: number, second: number): void {
  clearTimeout(rejectionTimer);
  rejection.value = {
    cells: new Set([first, second]),
    reason: rejectionReason(first, second),
    serial: ++rejectionSerial,
  };
  rejectionTimer = setTimeout(() => (rejection.value = null), REJECTION_SHOWN_MS);
}

/** `onRejected` replaces the error toast when the core refuses the command. */
async function send(
  command: MessageInitShape<typeof NumbersMatchCommandSchema>,
  onRejected?: () => void,
): Promise<void> {
  if (!canPlay.value || sending.value) return;
  sending.value = true;
  try {
    await props.controller.submit({ game: { case: "numbersMatch", value: command } });
  } catch (error) {
    if (error instanceof CommandRejectedError && onRejected) onRejected();
    else toasts.failure(error);
  } finally {
    sending.value = false;
  }
}

function select(index: number): void {
  if (!canPlay.value || sending.value) return;
  const first = selectedIndex.value;
  if (first === null || first === index) {
    selectedIndex.value = first === index ? null : index;
    return;
  }
  selectedIndex.value = null;
  // Like the original game: a cell that does not pair with the selection becomes the selection.
  void send({ command: { case: "removePair", value: { firstIndex: first, secondIndex: index } } }, () => {
    showRejection(first, index);
    selectedIndex.value = index;
  });
}

function refill(): void {
  void send({ command: { case: "refill", value: {} } });
}

function requestHint(): void {
  if (hintsEnabled.value) void send({ command: { case: "requestHint", value: {} } });
}

useGameKeys((key) => {
  // Observers and replays: the keys belong to the replay bar.
  if (props.controller.mySlot === null) return false;
  switch (key.toLowerCase()) {
    case "h":
      requestHint();
      return true;
    case "r":
      refill();
      return true;
    case "escape":
      selectedIndex.value = null;
      return true;
    default:
      return false;
  }
});
</script>

<template>
  <div v-if="view" class="flex flex-col items-center gap-2">
    <p class="text-sm font-medium" :class="status.tone">{{ status.text }}</p>
    <!-- The absolute scroller keeps a tall board from growing the page: it scrolls inside the
         space the layout leaves instead. -->
    <div class="relative min-h-48 w-full flex-1">
      <div class="absolute inset-0 overflow-y-auto overscroll-contain">
        <CellGrid
          :view="view"
          :selected-index="selectedIndex"
          :hinted-indices="hintedIndices"
          :rejected-indices="rejection?.cells ?? NO_CELLS"
          :rejection-serial="rejection?.serial ?? 0"
          :interactive="canPlay"
          @select="select"
        />
      </div>
    </div>
    <div class="flex items-center justify-center gap-8 pt-2 pb-1">
      <ActionButton
        caption="Refill"
        label="Refill (R)"
        icon="➕"
        :count="String(view.refillsLeft)"
        :disabled="!canPlay || view.refillsLeft === 0"
        :highlighted="suggestsRefill"
        @press="refill"
      />
      <ActionButton
        v-if="hintsEnabled"
        caption="Hint"
        label="Hint (H)"
        icon="💡"
        :count="hintsLeftText"
        :disabled="!canPlay || view.hintsLeft === 0"
        @press="requestHint"
      />
    </div>
  </div>
</template>
