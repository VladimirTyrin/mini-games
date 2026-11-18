<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";

import type { MatchController } from "@/match/match-controller";
import { type CheckersPiece, CheckersEndReason, CheckersSide } from "@/proto/minigames/v2/checkers_pb";
import { useToastStore } from "@/stores/toasts";
import { matchSeatNames } from "@/games/seat-names";

import { canStopHere, choosePath } from "./path-choice";

const props = defineProps<{ controller: MatchController }>();
const names = matchSeatNames(props.controller.source.header);
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
/** The house rule lets this capture end here although it could go on. */
const stoppable = computed(() => myTurn.value && canStopHere(myPaths.value, selection.value));

/** Per jump of a capture the opponent made, as it is played back. */
const STEP_MS = 220;
/** The opponent's piece walking its capture: where it stands for now. */
const walk = ref<{ id: number; square: number } | null>(null);
/** Pieces the walking piece has yet to jump: still shown until it has. */
const ghosts = ref<CheckersPiece[]>([]);
let walkTimers: ReturnType<typeof setTimeout>[] = [];

function stopWalk(): void {
  for (const timer of walkTimers) clearTimeout(timer);
  walkTimers = [];
  walk.value = null;
  ghosts.value = [];
}
onBeforeUnmount(stopWalk);

// Someone else's capture is played back jump by jump (a plain move just slides); the player's own
// was already walked while it was picked. Set before the new position is drawn, so the piece
// starts from where it was.
watch(view, (current, previous) => {
  const move = current?.lastMove;
  if (!previous || !move || move.path.join(",") === previous.lastMove?.path.join(",")) return;
  stopWalk();
  const path = move.path;
  const mover = current.pieces.find((piece) => piece.square === path.at(-1));
  if (!mover || mover.side === mySide.value || (path.length <= 2 && move.captured.length === 0)) return;
  ghosts.value = move.captured.flatMap((square) =>
    previous.pieces.filter((piece) => piece.square === square),
  );
  walk.value = { id: mover.id, square: path[1] ?? mover.square };
  for (let step = 1; step < path.length; step += 1) {
    walkTimers.push(
      setTimeout(() => {
        const jumpedSquare = move.captured[step - 1];
        ghosts.value = ghosts.value.filter((piece) => piece.square !== jumpedSquare);
        const next = path[step + 1];
        walk.value = next === undefined ? null : { id: mover.id, square: next };
      }, step * STEP_MS),
    );
  }
});

/** In id order: a piece keeps its place among the elements, so moving it is a transition. */
const shownPieces = computed(() =>
  [...(view.value?.pieces ?? []), ...ghosts.value].sort((first, second) => first.id - second.id),
);

/** Centres of the last move's squares, for its trail over the board (in eighths). */
const lastMoveTrail = computed(() =>
  (view.value?.lastMove?.path ?? [])
    .map((square) => {
      const { column, row } = screenCell(square);
      return `${column + 0.5},${row + 0.5}`;
    })
    .join(" "),
);

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

/**
 * Where a piece is drawn: a piece being moved stands on the last square picked for it, and the
 * opponent's capturing piece where its playback has got to.
 */
function shownSquare(piece: CheckersPiece): number {
  if (walk.value?.id === piece.id) return walk.value.square;
  return piece.square === selection.value[0] ? (selection.value.at(-1) ?? piece.square) : piece.square;
}

function screenCell(square: number): { column: number; row: number } {
  const x = square % 8;
  const y = Math.floor(square / 8);
  return flipped.value ? { column: 7 - x, row: y } : { column: x, row: 7 - y };
}

/** Moved with a transform (a share of the piece's own size), which slides on the GPU without layout. */
function screenPosition(square: number): { transform: string } {
  const { column, row } = screenCell(square);
  return { transform: `translate(${column * 100}%, ${row * 100}%)` };
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
  if (selection.value.includes(cell.square)) return "bg-[#5f7f35]";
  if (lastMoveSquares.value.has(cell.square)) return "bg-[#b7791f]";
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
      :style="{ width: 'min(100%, max(62dvh, calc(100dvh - 11rem)))' }"
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
      <svg
        v-if="view.lastMove"
        viewBox="0 0 8 8"
        class="pointer-events-none absolute inset-0 h-full w-full"
        aria-hidden="true"
      >
        <polyline
          :points="lastMoveTrail"
          fill="none"
          stroke="#fcd34d"
          stroke-opacity="0.75"
          stroke-width="0.12"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
      <!-- Pieces are drawn over the squares, keyed by id, so a move slides instead of jumping; the
           squares underneath take the taps. -->
      <TransitionGroup name="checkers-piece" tag="div" class="pointer-events-none absolute inset-0">
        <div
          v-for="piece in shownPieces"
          :key="piece.id"
          class="checkers-piece absolute top-0 left-0 h-[12.5%] w-[12.5%]"
          :style="screenPosition(shownSquare(piece))"
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
    <button
      v-if="stoppable"
      type="button"
      class="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-medium whitespace-nowrap text-white hover:bg-emerald-500"
      :disabled="sending"
      @click="submit(selection)"
    >
      End the move here
    </button>
    <p v-else-if="mustCapture" class="text-sm text-amber-300">You must capture</p>
    <p v-else-if="view.kingMoves > 0 && !view.end" class="text-sm text-gray-400">
      Kings only: {{ view.kingMoves }} of {{ view.kingMovesForDraw }} moves to a draw
    </p>
  </div>
</template>

<style scoped>
.checkers-piece {
  transition: transform 0.22s ease-out;
}
.checkers-piece-leave-active {
  transition: opacity 0.25s ease;
}
.checkers-piece-leave-to {
  opacity: 0;
}
</style>
