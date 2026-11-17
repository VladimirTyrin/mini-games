<script setup lang="ts">
import { computed, ref, watch } from "vue";

import type { MatchController } from "@/match/match-controller";
import { CheckersEndReason, CheckersSide } from "@/proto/minigames/v2/checkers_pb";
import { useToastStore } from "@/stores/toasts";
import { seatLabels } from "@/util/players";

import { choosePath } from "./path-choice";

const props = defineProps<{ controller: MatchController }>();
const names = seatLabels(props.controller.source.header);
const toasts = useToastStore();
const sending = ref(false);
/** The squares picked so far: the moving piece, then each landing. */
const selection = ref<number[]>([]);

const view = computed(() => {
  const game = props.controller.current.value?.view.game;
  return game?.case === "checkers" ? game.value : null;
});
const mySlot = computed(() => props.controller.mySlot);
const myTurn = computed(
  () => view.value?.currentSlot !== undefined && view.value.currentSlot === mySlot.value,
);
const mySide = computed(() => {
  if (!view.value || mySlot.value === null) return null;
  if (view.value.whiteSlot === mySlot.value) return CheckersSide.WHITE;
  if (view.value.blackSlot === mySlot.value) return CheckersSide.BLACK;
  return null;
});
/** Black sits at the bottom for the player of black; everybody else sees white at the bottom. */
const flipped = computed(() => mySide.value === CheckersSide.BLACK);
const myPaths = computed(() => (myTurn.value ? (view.value?.legalMoves ?? []).map((move) => move.path) : []));
const mustCapture = computed(() => myTurn.value && (view.value?.legalMoves[0]?.captured.length ?? 0) > 0);

// A new position (the opponent moved, a move was refused) starts the choice over; when one piece
// alone can move it is picked already.
watch(
  () => myPaths.value.map((path) => path.join(",")).join(";"),
  () => {
    const starts = [...new Set(myPaths.value.map((path) => path[0]))];
    const only = starts.length === 1 ? starts[0] : undefined;
    selection.value = only === undefined ? [] : [only];
  },
  { immediate: true },
);

const candidates = computed(() =>
  myPaths.value.filter((path) => selection.value.every((square, index) => path[index] === square)),
);
const movable = computed(() => new Set(myPaths.value.map((path) => path[0])));
const targets = computed(() => {
  const step = selection.value.length;
  if (step === 0) return new Set<number>();
  return new Set(candidates.value.map((path) => path[step]).filter((square) => square !== undefined));
});
/** Pieces already jumped in the half-picked capture: shown fading. */
const jumped = computed(() => {
  const steps = selection.value.length - 1;
  const move = view.value?.legalMoves.find((legal) =>
    selection.value.every((square, index) => legal.path[index] === square),
  );
  return new Set(steps > 0 && move ? move.captured.slice(0, steps) : []);
});
const lastMoveSquares = computed(() => new Set(view.value?.lastMove?.path ?? []));

interface Cell {
  square: number;
  dark: boolean;
}

/** Squares in screen order, top-left first. */
const cells = computed<Cell[]>(() => {
  const result: Cell[] = [];
  for (let row = 0; row < 8; row += 1) {
    for (let column = 0; column < 8; column += 1) {
      const { x, y } = boardCoordinates(row, column);
      result.push({ square: y * 8 + x, dark: (x + y) % 2 === 0 });
    }
  }
  return result;
});

function boardCoordinates(row: number, column: number): { x: number; y: number } {
  return flipped.value ? { x: 7 - column, y: row } : { x: column, y: 7 - row };
}

/** A piece being moved stands on the last square picked for it, so a capture is walked visibly. */
function shownSquare(square: number): number {
  return square === selection.value[0] ? (selection.value.at(-1) ?? square) : square;
}

function screenPosition(square: number): { left: string; top: string } {
  const x = square % 8;
  const y = Math.floor(square / 8);
  const column = flipped.value ? 7 - x : x;
  const row = flipped.value ? y : 7 - y;
  return { left: `${column * 12.5}%`, top: `${row * 12.5}%` };
}

function tap(square: number): void {
  if (!myTurn.value || sending.value) return;
  const choice = choosePath(myPaths.value, selection.value, square);
  switch (choice.kind) {
    case "ignore":
      return;
    case "partial":
      selection.value = choice.selection;
      return;
    case "complete":
      selection.value = choice.selection;
      void submit(choice.selection);
  }
}

async function submit(path: number[]): Promise<void> {
  sending.value = true;
  try {
    await props.controller.submit({
      game: { case: "checkers", value: { command: { case: "move", value: { path } } } },
    });
  } catch (error) {
    selection.value = [];
    toasts.failure(error);
  } finally {
    sending.value = false;
  }
}

function squareColor(cell: Cell): string {
  if (!cell.dark) return "bg-[#ecd9b8]";
  if (selection.value.includes(cell.square)) return "bg-[#b0893a]";
  if (lastMoveSquares.value.has(cell.square)) return "bg-[#a8743f]";
  return "bg-[#8b5a3c]";
}

function sideName(side: CheckersSide | null): string {
  return side === CheckersSide.BLACK ? "black" : "white";
}

const status = computed(() => {
  const current = view.value;
  if (!current) return "";
  const watching = mySlot.value === null;
  if (current.end) {
    const winner = current.end.winnerSlot;
    if (winner === undefined) return "Draw";
    if (watching) return `${names[winner] ?? "A player"} wins`;
    if (current.end.reason === CheckersEndReason.OPPONENT_LEFT && winner === mySlot.value)
      return "Opponent left — you win";
    return winner === mySlot.value ? "You win!" : "You lose";
  }
  if (current.currentSlot === undefined) return "Choosing who plays white…";
  if (watching) {
    const side = current.sideToMove === CheckersSide.BLACK ? "Black" : "White";
    return `${side} to move: ${names[current.currentSlot] ?? "a player"}`;
  }
  return myTurn.value ? "Your turn" : "Opponent's turn";
});
</script>

<template>
  <div v-if="view" class="flex w-full flex-col items-center gap-2">
    <p class="text-center text-lg" :class="myTurn ? 'text-emerald-300' : 'text-gray-300'">
      {{ status }}
      <span v-if="mySide !== null" class="whitespace-nowrap text-sm text-gray-400"
        >(you play {{ sideName(mySide) }})</span
      >
    </p>
    <div
      class="relative aspect-square overflow-hidden rounded-md border-4 border-[#3f2a1d] shadow-lg"
      :style="{ width: 'min(92vw, 62vh, 560px)' }"
    >
      <div class="grid h-full w-full grid-cols-8 grid-rows-8">
        <button
          v-for="cell in cells"
          :key="cell.square"
          type="button"
          class="relative min-h-0 min-w-0 disabled:cursor-default disabled:opacity-100"
          :class="squareColor(cell)"
          :disabled="!cell.dark || !myTurn"
          :aria-label="`${'abcdefgh'[cell.square % 8]}${Math.floor(cell.square / 8) + 1}`"
          @click="tap(cell.square)"
        >
          <span
            v-if="targets.has(cell.square)"
            class="absolute left-[35%] top-[35%] h-[30%] w-[30%] rounded-full bg-emerald-300/80"
          />
        </button>
      </div>
      <!-- Pieces are drawn over the squares, keyed by id, so a move slides instead of jumping; the
           squares underneath take the taps. -->
      <TransitionGroup name="checkers-piece" tag="div" class="pointer-events-none absolute inset-0">
        <div
          v-for="piece in view.pieces"
          :key="piece.id"
          class="checkers-piece absolute h-[12.5%] w-[12.5%]"
          :style="screenPosition(shownSquare(piece.square))"
        >
          <div
            class="absolute inset-[9%] flex items-center justify-center rounded-full border-2 shadow-md"
            :class="[
              piece.side === CheckersSide.WHITE
                ? 'border-[#c9bfae] bg-[#f5f0e6]'
                : 'border-[#555] bg-[#1f1f1f]',
              jumped.has(piece.square) ? 'opacity-40' : '',
              movable.has(piece.square) && selection.length === 0 ? 'ring-2 ring-emerald-300' : '',
              selection[0] === piece.square ? 'ring-2 ring-amber-300' : '',
            ]"
          >
            <svg v-if="piece.king" viewBox="0 0 24 24" class="h-3/5 w-3/5" aria-label="king">
              <path
                d="M3 18 5 7l4.5 5L12 5l2.5 7L19 7l2 11z"
                fill="#facc15"
                stroke="#a16207"
                stroke-width="1"
              />
            </svg>
            <span
              v-else
              class="h-1/2 w-1/2 rounded-full border"
              :class="piece.side === CheckersSide.WHITE ? 'border-[#c9bfae]' : 'border-[#555]'"
            />
          </div>
        </div>
      </TransitionGroup>
    </div>
    <p v-if="mustCapture" class="text-sm text-amber-300">You must capture</p>
    <p v-else-if="view.kingMoves > 0 && !view.end" class="text-sm text-gray-400">
      Kings only: {{ view.kingMoves }} of {{ view.kingMovesForDraw }} moves to a draw
    </p>
  </div>
</template>

<style scoped>
.checkers-piece {
  transition:
    left 0.25s ease,
    top 0.25s ease;
}
.checkers-piece-leave-active {
  transition: opacity 0.25s ease;
}
.checkers-piece-leave-to {
  opacity: 0;
}
</style>
