<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useGameStore, Puzzle2048Direction } from "../../stores/game";
import { Puzzle2048GameStatus } from "../../proto/games/puzzle2048_pb";

const gameStore = useGameStore();

const containerRef = ref<HTMLDivElement | null>(null);
const containerSize = ref({ width: 0, height: 0 });

const state = computed(() => gameStore.puzzle2048State);

const fieldWidth = computed(() => state.value?.fieldWidth ?? 4);
const fieldHeight = computed(() => state.value?.fieldHeight ?? 4);

const GAP_SIZE = 8;
const SLIDE_MS = 100;
const APPEAR_MS = 150;

const cellSize = computed(() => {
  if (!state.value || containerSize.value.width === 0) return 80;

  const padding = 16;
  const availableWidth = containerSize.value.width - padding * 2;
  const availableHeight = containerSize.value.height - 120;

  const totalGapsX = (fieldWidth.value + 1) * GAP_SIZE;
  const totalGapsY = (fieldHeight.value + 1) * GAP_SIZE;

  const cellByWidth = Math.floor((availableWidth - totalGapsX) / fieldWidth.value);
  const cellByHeight = Math.floor((availableHeight - totalGapsY) / fieldHeight.value);

  const size = Math.min(cellByWidth, cellByHeight);
  return Math.max(40, Math.min(100, size));
});

const fontSize = computed(() => {
  const size = cellSize.value;
  if (size >= 80) return "text-2xl";
  if (size >= 60) return "text-xl";
  if (size >= 45) return "text-lg";
  return "text-base";
});

const gameInProgress = computed(() => {
  return state.value?.status === Puzzle2048GameStatus.PUZZLE_2048_GAME_STATUS_IN_PROGRESS;
});

const statusText = computed(() => {
  if (!state.value) return "";
  switch (state.value.status) {
    case Puzzle2048GameStatus.PUZZLE_2048_GAME_STATUS_WON:
      return "You Win!";
    case Puzzle2048GameStatus.PUZZLE_2048_GAME_STATUS_LOST:
      return "Game Over";
    default:
      return "";
  }
});

function tileColor(value: number): string {
  const colors: Record<number, string> = {
    2: "bg-[#eee4da] text-[#776e65]",
    4: "bg-[#ede0c8] text-[#776e65]",
    8: "bg-[#f2b179] text-white",
    16: "bg-[#f59563] text-white",
    32: "bg-[#f67c5f] text-white",
    64: "bg-[#f65e3b] text-white",
    128: "bg-[#edcf72] text-white",
    256: "bg-[#edcc61] text-white",
    512: "bg-[#edc850] text-white",
    1024: "bg-[#edc53f] text-white",
    2048: "bg-[#edc22e] text-white",
    4096: "bg-[#3c3a32] text-white",
    8192: "bg-[#3c3a32] text-white",
  };
  return colors[value] ?? "bg-[#3c3a32] text-white";
}

// --- Tile management ---

interface Tile {
  id: number;
  value: number;
  row: number;
  col: number;
  isNew: boolean;
  isMergeResult: boolean;
}

const tiles = ref<Tile[]>([]);
let nextTileId = 0;
let isAnimating = false;
let pendingDirection: string | null = null;

function setTilesFromCells(cells: number[]): void {
  const w = fieldWidth.value;
  const newTiles: Tile[] = [];
  for (let i = 0; i < cells.length; i++) {
    const v = cells[i] ?? 0;
    if (v > 0) {
      newTiles.push({
        id: nextTileId++,
        value: v,
        row: Math.floor(i / w),
        col: i % w,
        isNew: false,
        isMergeResult: false,
      });
    }
  }
  tiles.value = newTiles;
}

// --- Move simulation ---

interface LineResult {
  outputPositions: number[];
  values: number[];
  mergePositions: Set<number>;
}

function processLine(values: number[]): LineResult {
  const n = values.length;
  const outputPositions = new Array<number>(n).fill(0);
  const resultValues: number[] = [];
  const mergePositions = new Set<number>();

  const nonZero: { value: number; origIdx: number }[] = [];
  for (let i = 0; i < n; i++) {
    if ((values[i] ?? 0) > 0) {
      nonZero.push({ value: values[i] ?? 0, origIdx: i });
    }
  }

  let i = 0;
  while (i < nonZero.length) {
    if (i + 1 < nonZero.length && nonZero[i]!.value === nonZero[i + 1]!.value) {
      const outIdx = resultValues.length;
      resultValues.push(nonZero[i]!.value * 2);
      outputPositions[nonZero[i]!.origIdx] = outIdx;
      outputPositions[nonZero[i + 1]!.origIdx] = outIdx;
      mergePositions.add(outIdx);
      i += 2;
    } else {
      const outIdx = resultValues.length;
      resultValues.push(nonZero[i]!.value);
      outputPositions[nonZero[i]!.origIdx] = outIdx;
      i++;
    }
  }

  while (resultValues.length < n) {
    resultValues.push(0);
  }

  return { outputPositions, values: resultValues, mergePositions };
}

interface MoveResult {
  changed: boolean;
  tileMovements: Map<number, { toRow: number; toCol: number; consumed: boolean }>;
  merges: { row: number; col: number; value: number }[];
}

function simulateMove(
  currentTiles: Tile[],
  width: number,
  height: number,
  direction: string,
): MoveResult {
  const cells = new Array(width * height).fill(0);
  const cellToTileId = new Map<number, number>();
  for (const tile of currentTiles) {
    const idx = tile.row * width + tile.col;
    cells[idx] = tile.value;
    cellToTileId.set(idx, tile.id);
  }

  const tileMovements = new Map<number, { toRow: number; toCol: number; consumed: boolean }>();
  const merges: { row: number; col: number; value: number }[] = [];
  const resultCells = new Array(width * height).fill(0);

  const isHorizontal = direction === "left" || direction === "right";
  const lineCount = isHorizontal ? height : width;
  const lineLength = isHorizontal ? width : height;

  for (let lineIdx = 0; lineIdx < lineCount; lineIdx++) {
    const lineValues: number[] = [];
    const lineTileIds: (number | null)[] = [];

    for (let pos = 0; pos < lineLength; pos++) {
      let row: number, col: number;
      if (direction === "left") { row = lineIdx; col = pos; }
      else if (direction === "right") { row = lineIdx; col = width - 1 - pos; }
      else if (direction === "up") { row = pos; col = lineIdx; }
      else { row = height - 1 - pos; col = lineIdx; }

      const idx = row * width + col;
      lineValues.push(cells[idx] as number);
      lineTileIds.push(cellToTileId.get(idx) ?? null);
    }

    const result = processLine(lineValues);
    const occupiedOutputs = new Set<number>();

    for (let pos = 0; pos < lineLength; pos++) {
      const tileId = lineTileIds[pos] ?? null;
      if (tileId === null || (lineValues[pos] ?? 0) === 0) continue;

      const outPos = result.outputPositions[pos] ?? 0;
      let toRow: number, toCol: number;
      if (direction === "left") { toRow = lineIdx; toCol = outPos; }
      else if (direction === "right") { toRow = lineIdx; toCol = width - 1 - outPos; }
      else if (direction === "up") { toRow = outPos; toCol = lineIdx; }
      else { toRow = height - 1 - outPos; toCol = lineIdx; }

      const consumed = occupiedOutputs.has(outPos);
      occupiedOutputs.add(outPos);

      tileMovements.set(tileId, { toRow, toCol, consumed });
    }

    for (let outPos = 0; outPos < lineLength; outPos++) {
      let row: number, col: number;
      if (direction === "left") { row = lineIdx; col = outPos; }
      else if (direction === "right") { row = lineIdx; col = width - 1 - outPos; }
      else if (direction === "up") { row = outPos; col = lineIdx; }
      else { row = height - 1 - outPos; col = lineIdx; }

      resultCells[row * width + col] = result.values[outPos] ?? 0;

      if (result.mergePositions.has(outPos)) {
        merges.push({ row, col, value: result.values[outPos] ?? 0 });
      }
    }
  }

  const changed = cells.some((v, i) => v !== resultCells[i]);
  return { changed, tileMovements, merges };
}

function directionToString(dir: Puzzle2048Direction): string {
  switch (dir) {
    case Puzzle2048Direction.PUZZLE_2048_DIRECTION_UP: return "up";
    case Puzzle2048Direction.PUZZLE_2048_DIRECTION_DOWN: return "down";
    case Puzzle2048Direction.PUZZLE_2048_DIRECTION_LEFT: return "left";
    case Puzzle2048Direction.PUZZLE_2048_DIRECTION_RIGHT: return "right";
    default: return "up";
  }
}

// --- Animation ---

function sendMove(dir: Puzzle2048Direction): void {
  if (isAnimating || !gameInProgress.value) return;

  const dirStr = directionToString(dir);
  const result = simulateMove(tiles.value, fieldWidth.value, fieldHeight.value, dirStr);
  if (!result.changed) return;

  pendingDirection = dirStr;
  isAnimating = true;
  gameStore.sendPuzzle2048Move(dir);

  setTimeout(() => {
    if (pendingDirection !== null) {
      pendingDirection = null;
      isAnimating = false;
    }
  }, 3000);
}

function addSpawnAndReconcile(serverCells: number[]): void {
  const w = fieldWidth.value;
  const ourGrid = new Array(serverCells.length).fill(0);
  for (const tile of tiles.value) {
    ourGrid[tile.row * w + tile.col] = tile.value;
  }

  for (let i = 0; i < serverCells.length; i++) {
    const sv = serverCells[i] ?? 0;
    if (sv > 0 && ourGrid[i] === 0) {
      tiles.value.push({
        id: nextTileId++,
        value: sv,
        row: Math.floor(i / w),
        col: i % w,
        isNew: true,
        isMergeResult: false,
      });
    }
  }

  setTimeout(() => {
    for (const tile of tiles.value) {
      tile.isNew = false;
      tile.isMergeResult = false;
    }
    isAnimating = false;
  }, APPEAR_MS);
}

watch(
  () => state.value?.cells ? [...state.value.cells] : null,
  (newCells: number[] | null) => {
    if (!newCells || newCells.length === 0) {
      tiles.value = [];
      return;
    }

    const dir = pendingDirection;
    pendingDirection = null;

    if (dir && tiles.value.length > 0) {
      const result = simulateMove(tiles.value, fieldWidth.value, fieldHeight.value, dir);

      if (!result.changed) {
        setTilesFromCells(newCells);
        isAnimating = false;
        return;
      }

      for (const tile of tiles.value) {
        const mv = result.tileMovements.get(tile.id);
        if (mv) {
          tile.row = mv.toRow;
          tile.col = mv.toCol;
        }
      }

      setTimeout(() => {
        tiles.value = tiles.value.filter(t => {
          const mv = result.tileMovements.get(t.id);
          return !mv || !mv.consumed;
        });

        for (const merge of result.merges) {
          const tile = tiles.value.find(t => t.row === merge.row && t.col === merge.col);
          if (tile) {
            tile.value = merge.value;
            tile.isMergeResult = true;
          }
        }

        addSpawnAndReconcile(newCells);
      }, SLIDE_MS);
    } else {
      setTilesFromCells(newCells);
      isAnimating = false;
    }
  },
  { deep: true },
);

// --- Score ---

const scoreDisplay = ref(0);
const scoreBump = ref(false);

watch(
  () => state.value?.score,
  (newScore) => {
    if (newScore === undefined) return;
    if (newScore > scoreDisplay.value) {
      scoreBump.value = true;
      setTimeout(() => { scoreBump.value = false; }, 300);
    }
    scoreDisplay.value = newScore;
  },
);

// --- Board style ---

const boardStyle = computed(() => {
  const w = fieldWidth.value * cellSize.value + (fieldWidth.value + 1) * GAP_SIZE;
  const h = fieldHeight.value * cellSize.value + (fieldHeight.value + 1) * GAP_SIZE;
  return {
    position: "relative" as const,
    width: `${w}px`,
    height: `${h}px`,
  };
});

function cellBgStyle(index: number) {
  const row = Math.floor(index / fieldWidth.value);
  const col = index % fieldWidth.value;
  return {
    position: "absolute" as const,
    width: `${cellSize.value}px`,
    height: `${cellSize.value}px`,
    left: `${GAP_SIZE + col * (cellSize.value + GAP_SIZE)}px`,
    top: `${GAP_SIZE + row * (cellSize.value + GAP_SIZE)}px`,
  };
}

function tileTransform(tile: Tile): string {
  const x = GAP_SIZE + tile.col * (cellSize.value + GAP_SIZE);
  const y = GAP_SIZE + tile.row * (cellSize.value + GAP_SIZE);
  return `translate(${x}px, ${y}px)`;
}

// --- Input handling ---

let touchStartX = 0;
let touchStartY = 0;

function handleTouchStart(event: TouchEvent): void {
  if (!gameInProgress.value) return;
  const touch = event.touches[0];
  if (!touch) return;
  touchStartX = touch.clientX;
  touchStartY = touch.clientY;
}

function handleTouchEnd(event: TouchEvent): void {
  if (!gameInProgress.value) return;
  const touch = event.changedTouches[0];
  if (!touch) return;
  const dx = touch.clientX - touchStartX;
  const dy = touch.clientY - touchStartY;

  const minSwipe = 30;
  if (Math.abs(dx) < minSwipe && Math.abs(dy) < minSwipe) return;

  event.preventDefault();

  if (Math.abs(dx) > Math.abs(dy)) {
    sendMove(dx > 0
      ? Puzzle2048Direction.PUZZLE_2048_DIRECTION_RIGHT
      : Puzzle2048Direction.PUZZLE_2048_DIRECTION_LEFT);
  } else {
    sendMove(dy > 0
      ? Puzzle2048Direction.PUZZLE_2048_DIRECTION_DOWN
      : Puzzle2048Direction.PUZZLE_2048_DIRECTION_UP);
  }
}

function handleKeyDown(event: KeyboardEvent): void {
  if (!gameInProgress.value) return;

  let direction: Puzzle2048Direction | null = null;

  switch (event.key) {
    case "ArrowUp":
    case "w":
    case "W":
      direction = Puzzle2048Direction.PUZZLE_2048_DIRECTION_UP;
      break;
    case "ArrowDown":
    case "s":
    case "S":
      direction = Puzzle2048Direction.PUZZLE_2048_DIRECTION_DOWN;
      break;
    case "ArrowLeft":
    case "a":
    case "A":
      direction = Puzzle2048Direction.PUZZLE_2048_DIRECTION_LEFT;
      break;
    case "ArrowRight":
    case "d":
    case "D":
      direction = Puzzle2048Direction.PUZZLE_2048_DIRECTION_RIGHT;
      break;
  }

  if (direction !== null) {
    event.preventDefault();
    sendMove(direction);
  }
}

// --- Lifecycle ---

let resizeObserver: ResizeObserver | null = null;

function updateContainerSize(): void {
  if (containerRef.value) {
    containerSize.value = {
      width: containerRef.value.clientWidth,
      height: window.innerHeight - 80,
    };
  }
}

onMounted(() => {
  updateContainerSize();
  resizeObserver = new ResizeObserver(() => {
    updateContainerSize();
  });
  if (containerRef.value) {
    resizeObserver.observe(containerRef.value);
  }
  window.addEventListener("keydown", handleKeyDown);
  if (state.value) {
    scoreDisplay.value = state.value.score;
    setTilesFromCells([...state.value.cells]);
  }
});

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect();
    resizeObserver = null;
  }
  window.removeEventListener("keydown", handleKeyDown);
});
</script>

<template>
  <div
    ref="containerRef"
    class="flex flex-col items-center w-full max-w-lg mx-auto px-2"
    @touchstart="handleTouchStart"
    @touchend="handleTouchEnd"
  >
    <div class="w-full flex justify-between items-center py-3">
      <div
        class="score-box"
        :class="{ 'score-bump': scoreBump }"
      >
        <div class="text-xs uppercase tracking-wider text-[#eee4da99]">Score</div>
        <div class="text-2xl font-bold text-white">{{ scoreDisplay }}</div>
      </div>
      <span
        v-if="statusText"
        class="font-bold text-xl status-appear"
        :class="{
          'text-[#edc22e]': state?.status === Puzzle2048GameStatus.PUZZLE_2048_GAME_STATUS_WON,
          'text-red-400': state?.status === Puzzle2048GameStatus.PUZZLE_2048_GAME_STATUS_LOST,
        }"
      >
        {{ statusText }}
      </span>
      <div class="score-box">
        <div class="text-xs uppercase tracking-wider text-[#eee4da99]">Target</div>
        <div class="text-2xl font-bold text-white">{{ state?.targetValue ?? 2048 }}</div>
      </div>
    </div>

    <div v-if="state" class="flex-1 flex items-center justify-center py-2">
      <div class="board" :style="boardStyle">
        <div
          v-for="i in fieldWidth * fieldHeight"
          :key="'bg-' + i"
          class="cell-bg"
          :style="cellBgStyle(i - 1)"
        />

        <div
          v-for="tile in tiles"
          :key="tile.id"
          class="tile-wrapper"
          :style="{ transform: tileTransform(tile), zIndex: tile.isMergeResult ? 2 : 1 }"
        >
          <div
            class="tile"
            :class="[
              tileColor(tile.value),
              fontSize,
              { 'tile-new': tile.isNew, 'tile-merged': tile.isMergeResult },
            ]"
            :style="{ width: cellSize + 'px', height: cellSize + 'px' }"
          >
            {{ tile.value }}
          </div>
        </div>
      </div>
    </div>

    <div class="text-sm text-gray-500 py-2">
      Use arrow keys or WASD to move tiles
    </div>
  </div>
</template>

<style scoped>
.board {
  background: #574e44;
  border-radius: 6px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
}

.cell-bg {
  background: #49423a;
  border-radius: 4px;
}

.tile-wrapper {
  position: absolute;
  left: 0;
  top: 0;
  transition: transform 100ms ease-in-out;
}

.tile {
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  user-select: none;
}

.tile-new {
  animation: tile-appear 150ms ease-out;
}

.tile-merged {
  animation: tile-pop 200ms ease-out;
}

.score-box {
  background: #574e44;
  padding: 8px 20px;
  border-radius: 6px;
  text-align: center;
  min-width: 80px;
  transition: transform 150ms ease;
}

.score-bump {
  animation: score-bump 300ms ease-out;
}

.status-appear {
  animation: status-appear 400ms ease-out;
}

@keyframes tile-appear {
  0% {
    transform: scale(0);
    opacity: 0;
  }
  50% {
    transform: scale(1.1);
    opacity: 1;
  }
  100% {
    transform: scale(1);
  }
}

@keyframes tile-pop {
  0% {
    transform: scale(1);
  }
  40% {
    transform: scale(1.2);
  }
  100% {
    transform: scale(1);
  }
}

@keyframes score-bump {
  0% {
    transform: scale(1);
  }
  50% {
    transform: scale(1.15);
  }
  100% {
    transform: scale(1);
  }
}

@keyframes status-appear {
  0% {
    transform: scale(0.5);
    opacity: 0;
  }
  60% {
    transform: scale(1.1);
  }
  100% {
    transform: scale(1);
    opacity: 1;
  }
}
</style>
