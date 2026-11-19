<script setup lang="ts">
import type { MessageInitShape } from "@bufbuild/protobuf";
import { computed, onBeforeUnmount, ref, watch } from "vue";

import { t } from "@/i18n";
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
import { refusalText } from "@/util/refusal-text";

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
type Command = MessageInitShape<typeof NumbersMatchCommandSchema>;
const REFILL: Command = { command: { case: "refill", value: {} } };
const HINT: Command = { command: { case: "requestHint", value: {} } };

/** The rules' answer for the state on screen, in a sentence; `null` when they accept. */
function refusalOf(command: Command): string | null {
  const refusal = props.controller.refusal({ game: { case: "numbersMatch", value: command } });
  return refusal === null ? null : refusalText(refusal);
}

// Asked of the rules, so the buttons are off exactly when the game would refuse.
const canRefill = computed(() => refusalOf(REFILL) === null);
const canHint = computed(() => hintsEnabled.value && refusalOf(HINT) === null);
const activeCellCount = computed(
  () => view.value?.cells.filter((cell) => cell.state === NumbersMatchCellState.ACTIVE).length ?? 0,
);

const status = computed(() => {
  if (rejection.value) return { text: rejection.value.reason, tone: "text-rose-300" };
  const text = t.games.numbersMatch.board;
  const current = view.value;
  switch (current?.phase) {
    case NumbersMatchPhase.WON:
      return { text: text.cleared, tone: "text-emerald-300" };
    case NumbersMatchPhase.LOST:
      return current.lossReason === NumbersMatchLossReason.DROPPED
        ? { text: text.abandoned, tone: "text-gray-400" }
        : { text: text.noMovesLeft, tone: "text-rose-400" };
    case NumbersMatchPhase.PLAYING:
      return suggestsRefill.value
        ? { text: text.noPairsRefill, tone: "text-amber-300" }
        : { text: text.numbersLeft(activeCellCount.value), tone: "text-gray-400" };
    default:
      return { text: text.dealing, tone: "text-gray-400" };
  }
});

// Indices refer to the board they were picked on; any change except a shown hint can shift them.
watch(view, (current) => {
  if (current?.lastAction?.action.case !== "hintShown") selectedIndex.value = null;
  rejection.value = null;
});

function showRejection(first: number, second: number, reason: string): void {
  clearTimeout(rejectionTimer);
  rejection.value = {
    cells: new Set([first, second]),
    reason,
    serial: ++rejectionSerial,
  };
  rejectionTimer = setTimeout(() => (rejection.value = null), REJECTION_SHOWN_MS);
}

/** `onRejected` replaces the error toast when the core refuses the command. */
async function send(command: Command, onRejected?: () => void): Promise<void> {
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
  const removal: Command = {
    command: { case: "removePair", value: { firstIndex: first, secondIndex: index } },
  };
  // Like the original game: a cell that does not pair with the selection becomes the selection.
  const refuse = (reason: string) => {
    showRejection(first, index, reason);
    selectedIndex.value = index;
  };
  const refusal = refusalOf(removal);
  if (refusal !== null) {
    refuse(refusal);
    return;
  }
  // The authority may still refuse (the screen can be a frame behind): ask again for the reason.
  void send(removal, () => refuse(refusalOf(removal) ?? t.games.numbersMatch.board.pairRefused));
}

function refill(): void {
  if (canRefill.value) void send(REFILL);
}

function requestHint(): void {
  if (canHint.value) void send(HINT);
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
  <div
    v-if="view"
    class="flex flex-col items-center gap-2 short-landscape:flex-row short-landscape:items-stretch"
  >
    <!-- Two lines reserved: a longer message (why a pair was refused) must not push the board. -->
    <p class="flex min-h-10 items-center text-center text-sm leading-5 font-medium" :class="status.tone">
      {{ status.text }}
    </p>
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
    <div
      class="flex items-center justify-center gap-8 pt-2 pb-1 short-landscape:flex-col short-landscape:gap-4"
    >
      <ActionButton
        :caption="t.games.numbersMatch.board.refill"
        :label="t.games.numbersMatch.board.refillWithKey"
        icon="➕"
        :count="String(view.refillsLeft)"
        :disabled="!canRefill"
        :highlighted="suggestsRefill"
        @press="refill"
      />
      <ActionButton
        v-if="hintsEnabled"
        :caption="t.games.numbersMatch.board.hint"
        :label="t.games.numbersMatch.board.hintWithKey"
        icon="💡"
        :count="hintsLeftText"
        :disabled="!canHint"
        @press="requestHint"
      />
    </div>
  </div>
</template>
