<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useGameStore, HorizontalDirection } from "../../stores/game";
import { useConnectionStore } from "../../stores/connection";
import { useDeviceStore } from "../../stores/device";
import { WorkerState, GameStatus } from "../../proto/games/stack_attack_pb";

const gameStore = useGameStore();
const connectionStore = useConnectionStore();
const deviceStore = useDeviceStore();

const canvasRef = ref<HTMLCanvasElement | null>(null);
const containerRef = ref<HTMLDivElement | null>(null);
const containerSize = ref({ width: 0, height: 0 });

const BASE_CELL_SIZE = 40;
const MIN_CELL_SIZE = 20;
const MAX_CELL_SIZE = 60;

const COLORS = {
  background: "#2d3748",
  ground: "#4a5568",
  box: "#ed8936",
  boxPattern: "#dd6b20",
  crane: "#718096",
  craneHook: "#a0aec0",
  ceiling: "#1a202c",
};

const WORKER_COLORS = [
  "#48bb78",
  "#4299e1",
  "#ed64a6",
  "#ecc94b",
];

interface VisualPosition {
  x: number;
  y: number;
}

const workerVisuals = new Map<string, VisualPosition>();
const boxVisuals = new Map<number, VisualPosition>();
const craneVisuals = new Map<number, VisualPosition>();
let animationFrameId: number | null = null;

const LERP_SPEED = 0.25;

const state = computed(() => gameStore.stackAttackState);

const cellSize = computed(() => {
  if (!state.value || containerSize.value.width === 0) return BASE_CELL_SIZE;

  const availableWidth = containerSize.value.width - 200 - 24;
  const availableHeight = containerSize.value.height - 80;

  const cellByWidth = Math.floor(availableWidth / state.value.fieldWidth);
  const cellByHeight = Math.floor(availableHeight / (state.value.fieldHeight + 2));

  const size = Math.min(cellByWidth, cellByHeight);
  return Math.max(MIN_CELL_SIZE, Math.min(MAX_CELL_SIZE, size));
});

const canvasWidth = computed(() => {
  if (!state.value) return 400;
  return state.value.fieldWidth * cellSize.value;
});

const canvasHeight = computed(() => {
  if (!state.value) return 400;
  return (state.value.fieldHeight + 2) * cellSize.value;
});

const isGameOver = computed(() => {
  if (!state.value) return false;
  return state.value.status === GameStatus.GAME_OVER;
});

const myWorker = computed(() => {
  if (!state.value || !connectionStore.clientId) return null;
  return state.value.workers.find(
    (w) => w.playerId === connectionStore.clientId
  );
});

const isDead = computed(() => {
  if (!myWorker.value) return false;
  return !myWorker.value.alive;
});

function getWorkerColor(colorIndex: number): string {
  return WORKER_COLORS[colorIndex % WORKER_COLORS.length] ?? WORKER_COLORS[0]!;
}

function lerp(current: number, target: number, speed: number): number {
  const diff = target - current;
  if (Math.abs(diff) < 0.01) return target;
  return current + diff * speed;
}

function updateVisuals(): void {
  if (!state.value) return;

  const currentWorkerIds = new Set<string>();
  for (const worker of state.value.workers) {
    currentWorkerIds.add(worker.playerId);
    const visual = workerVisuals.get(worker.playerId);
    if (visual) {
      visual.x = lerp(visual.x, worker.x, LERP_SPEED);
      visual.y = lerp(visual.y, worker.y, LERP_SPEED);
    } else {
      workerVisuals.set(worker.playerId, { x: worker.x, y: worker.y });
    }
  }
  for (const id of workerVisuals.keys()) {
    if (!currentWorkerIds.has(id)) {
      workerVisuals.delete(id);
    }
  }

  const currentBoxIds = new Set<number>();
  for (const box of state.value.boxes) {
    currentBoxIds.add(box.id);
    const visual = boxVisuals.get(box.id);
    if (visual) {
      visual.x = lerp(visual.x, box.x, LERP_SPEED);
      visual.y = lerp(visual.y, box.y, LERP_SPEED);
    } else {
      boxVisuals.set(box.id, { x: box.x, y: box.y });
    }
  }
  for (const id of boxVisuals.keys()) {
    if (!currentBoxIds.has(id)) {
      boxVisuals.delete(id);
    }
  }

  const currentCraneIds = new Set<number>();
  for (const crane of state.value.cranes) {
    currentCraneIds.add(crane.id);
    const visual = craneVisuals.get(crane.id);
    if (visual) {
      visual.x = lerp(visual.x, crane.x, LERP_SPEED);
    } else {
      craneVisuals.set(crane.id, { x: crane.x, y: 0 });
    }
  }
  for (const id of craneVisuals.keys()) {
    if (!currentCraneIds.has(id)) {
      craneVisuals.delete(id);
    }
  }
}

function draw(): void {
  const canvas = canvasRef.value;
  const ctx = canvas?.getContext("2d");
  if (!canvas || !ctx || !state.value) return;

  const size = cellSize.value;
  const fieldHeight = state.value.fieldHeight;

  ctx.fillStyle = COLORS.background;
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  ctx.fillStyle = COLORS.ceiling;
  ctx.fillRect(0, 0, canvas.width, size);

  ctx.fillStyle = COLORS.ground;
  ctx.fillRect(0, (fieldHeight + 1) * size, canvas.width, size);

  drawCranes(ctx);
  drawBoxes(ctx);
  drawWorkers(ctx);
}

function drawCranes(ctx: CanvasRenderingContext2D): void {
  if (!state.value) return;

  const size = cellSize.value;

  for (const crane of state.value.cranes) {
    const visual = craneVisuals.get(crane.id);
    const x = visual?.x ?? crane.x;

    ctx.fillStyle = COLORS.crane;
    ctx.fillRect(
      x * size + size * 0.2,
      size * 0.2,
      size * 0.6,
      size * 0.6
    );

    ctx.fillStyle = COLORS.craneHook;
    ctx.fillRect(
      x * size + size * 0.4,
      size * 0.8,
      size * 0.2,
      size * 0.4
    );

    ctx.fillStyle = COLORS.box;
    ctx.fillRect(
      x * size + size * 0.1,
      size * 1.2,
      size * 0.8,
      size * 0.6
    );
  }
}

function drawBoxPattern(ctx: CanvasRenderingContext2D, bx: number, by: number, size: number, patternId: number): void {
  const p = size * 0.12;
  const m = size * 0.2;

  switch (patternId % 8) {
    case 0:
      ctx.fillRect(bx + m, by + m, p, p);
      ctx.fillRect(bx + size - m - p, by + m, p, p);
      ctx.fillRect(bx + m, by + size - m - p, p, p);
      ctx.fillRect(bx + size - m - p, by + size - m - p, p, p);
      break;
    case 1:
      ctx.fillRect(bx + size / 2 - p / 2, by + m, p, p);
      ctx.fillRect(bx + size / 2 - p / 2, by + size - m - p, p, p);
      break;
    case 2:
      ctx.fillRect(bx + m, by + size / 2 - p / 2, p, p);
      ctx.fillRect(bx + size - m - p, by + size / 2 - p / 2, p, p);
      break;
    case 3:
      ctx.fillRect(bx + size / 2 - p / 2, by + size / 2 - p / 2, p, p);
      break;
    case 4:
      ctx.fillRect(bx + m, by + m, size - m * 2, p);
      ctx.fillRect(bx + m, by + size - m - p, size - m * 2, p);
      break;
    case 5:
      ctx.fillRect(bx + m, by + m, p, size - m * 2);
      ctx.fillRect(bx + size - m - p, by + m, p, size - m * 2);
      break;
    case 6:
      ctx.beginPath();
      ctx.moveTo(bx + m, by + m);
      ctx.lineTo(bx + size - m, by + size - m);
      ctx.moveTo(bx + size - m, by + m);
      ctx.lineTo(bx + m, by + size - m);
      ctx.strokeStyle = COLORS.boxPattern;
      ctx.lineWidth = p * 0.8;
      ctx.stroke();
      break;
    case 7:
      ctx.fillRect(bx + m, by + m, p, p);
      ctx.fillRect(bx + size / 2 - p / 2, by + m, p, p);
      ctx.fillRect(bx + size - m - p, by + m, p, p);
      ctx.fillRect(bx + m, by + size / 2 - p / 2, p, p);
      ctx.fillRect(bx + size - m - p, by + size / 2 - p / 2, p, p);
      ctx.fillRect(bx + m, by + size - m - p, p, p);
      ctx.fillRect(bx + size / 2 - p / 2, by + size - m - p, p, p);
      ctx.fillRect(bx + size - m - p, by + size - m - p, p, p);
      break;
  }
}

function drawBoxes(ctx: CanvasRenderingContext2D): void {
  if (!state.value) return;

  const size = cellSize.value;
  const offsetY = size;

  for (const box of state.value.boxes) {
    const visual = boxVisuals.get(box.id);
    const visualX = visual?.x ?? box.x;
    const visualY = visual?.y ?? box.y;

    const bx = visualX * size;
    const by = (state.value.fieldHeight - 1 - visualY) * size + offsetY;

    ctx.fillStyle = COLORS.box;
    ctx.fillRect(bx + 2, by + 2, size - 4, size - 4);

    ctx.fillStyle = COLORS.boxPattern;
    drawBoxPattern(ctx, bx, by, size, box.patternId);

    if (box.falling) {
      ctx.strokeStyle = "#fff";
      ctx.lineWidth = 2;
      ctx.strokeRect(bx + 2, by + 2, size - 4, size - 4);
    }
  }
}

function drawWorkers(ctx: CanvasRenderingContext2D): void {
  if (!state.value) return;

  const size = cellSize.value;
  const offsetY = size;

  for (const worker of state.value.workers) {
    if (!worker.alive) continue;

    const visual = workerVisuals.get(worker.playerId);
    const visualX = visual?.x ?? worker.x;
    const visualY = visual?.y ?? worker.y;

    const x = visualX * size;
    const feetY = (state.value.fieldHeight - 1 - visualY) * size + offsetY;
    const headY = feetY - size;

    const color = getWorkerColor(worker.colorIndex);
    ctx.fillStyle = color;

    ctx.beginPath();
    ctx.arc(x + size / 2, headY + size * 0.4, size * 0.35, 0, Math.PI * 2);
    ctx.fill();

    ctx.fillRect(x + size * 0.25, headY + size * 0.75, size * 0.5, size * 0.5);

    ctx.fillRect(x + size * 0.3, feetY + size * 0.2, size * 0.4, size * 0.5);

    ctx.fillRect(x + size * 0.2, feetY + size * 0.7, size * 0.25, size * 0.25);
    ctx.fillRect(x + size * 0.55, feetY + size * 0.7, size * 0.25, size * 0.25);

    if (worker.state === WorkerState.JUMPING || worker.state === WorkerState.FALLING) {
      ctx.strokeStyle = "#fff";
      ctx.lineWidth = 2;
      ctx.strokeRect(x + size * 0.1, headY + size * 0.1, size * 0.8, size * 1.8);
    }

    if (worker.playerId === connectionStore.clientId) {
      ctx.fillStyle = "#fff";
      ctx.font = `bold ${Math.max(10, size * 0.3)}px sans-serif`;
      ctx.textAlign = "center";
      ctx.fillText("▼", x + size / 2, headY - 5);
    }
  }
}

function animate(): void {
  updateVisuals();
  draw();
  animationFrameId = requestAnimationFrame(animate);
}

function stopAnimation(): void {
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId);
    animationFrameId = null;
  }
}

function handleKeyDown(event: KeyboardEvent): void {
  if (isGameOver.value || isDead.value) return;

  switch (event.key) {
    case "ArrowLeft":
    case "a":
    case "A":
      event.preventDefault();
      gameStore.sendStackAttackMove(HorizontalDirection.LEFT);
      break;
    case "ArrowRight":
    case "d":
    case "D":
      event.preventDefault();
      gameStore.sendStackAttackMove(HorizontalDirection.RIGHT);
      break;
    case "ArrowUp":
    case "w":
    case "W":
    case " ":
      event.preventDefault();
      gameStore.sendStackAttackJump();
      break;
  }
}

const SWIPE_THRESHOLD = 30;
let touchStartX = 0;
let touchStartY = 0;

function handleTouchStart(event: TouchEvent): void {
  const touch = event.touches[0];
  if (touch) {
    touchStartX = touch.clientX;
    touchStartY = touch.clientY;
  }
}

function handleTouchEnd(event: TouchEvent): void {
  if (isGameOver.value || isDead.value) return;

  const touch = event.changedTouches[0];
  if (!touch) return;

  const deltaX = touch.clientX - touchStartX;
  const deltaY = touch.clientY - touchStartY;

  const absX = Math.abs(deltaX);
  const absY = Math.abs(deltaY);

  if (absX < SWIPE_THRESHOLD && absY < SWIPE_THRESHOLD) {
    gameStore.sendStackAttackJump();
    return;
  }

  if (absX > absY) {
    if (deltaX > 0) {
      gameStore.sendStackAttackMove(HorizontalDirection.RIGHT);
    } else {
      gameStore.sendStackAttackMove(HorizontalDirection.LEFT);
    }
  } else if (deltaY < 0) {
    gameStore.sendStackAttackJump();
  }
}

function formatPlayerName(playerId: string, isBot: boolean): string {
  if (isBot) return `${playerId} [BOT]`;
  return playerId;
}

let resizeObserver: ResizeObserver | null = null;

function updateContainerSize(): void {
  if (containerRef.value) {
    containerSize.value = {
      width: containerRef.value.clientWidth,
      height: window.innerHeight - 120,
    };
  }
}

onMounted(() => {
  window.addEventListener("keydown", handleKeyDown);

  updateContainerSize();
  resizeObserver = new ResizeObserver(() => {
    updateContainerSize();
  });
  if (containerRef.value) {
    resizeObserver.observe(containerRef.value);
  }

  animate();
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeyDown);
  stopAnimation();
  if (resizeObserver) {
    resizeObserver.disconnect();
    resizeObserver = null;
  }
});
</script>

<template>
  <div ref="containerRef" class="flex flex-col lg:flex-row gap-6 items-center lg:items-start">
    <div class="flex-shrink-0">
      <div
        v-if="isDead"
        class="mb-2 p-3 bg-red-900/80 border border-red-500 rounded-lg text-center"
        :style="{ width: canvasWidth + 'px' }"
      >
        <span class="text-red-200 font-bold">YOU ARE DEAD</span>
        <span class="text-red-300 text-sm ml-2">Watching the game...</span>
      </div>

      <div
        class="border-2 border-gray-600 rounded-lg overflow-hidden"
        :style="{ width: canvasWidth + 'px' }"
      >
        <canvas
          ref="canvasRef"
          :width="canvasWidth"
          :height="canvasHeight"
          class="block touch-none"
          @touchstart="handleTouchStart"
          @touchend="handleTouchEnd"
        />
      </div>

      <div v-if="!deviceStore.isTouchDevice" class="mt-4 text-center text-gray-400 text-sm">
        Use <span class="font-mono bg-gray-700 px-1 rounded">Arrow Keys</span>
        or
        <span class="font-mono bg-gray-700 px-1 rounded">WASD</span>
        to move,
        <span class="font-mono bg-gray-700 px-1 rounded">Space</span>
        to jump
      </div>
    </div>

    <div class="flex-grow min-w-52">
      <div class="bg-gray-800 rounded-lg p-4">
        <h3 class="text-lg font-semibold mb-4 text-gray-200">Game Info</h3>

        <div v-if="state" class="space-y-3">
          <div class="flex justify-between text-lg">
            <span class="text-gray-400">Score:</span>
            <span class="font-bold text-green-400">{{ state.score }}</span>
          </div>

          <div class="flex justify-between">
            <span class="text-gray-400">Lines Cleared:</span>
            <span class="font-mono text-gray-200">{{ state.linesCleared }}</span>
          </div>

          <div class="flex justify-between">
            <span class="text-gray-400">Difficulty:</span>
            <span class="font-mono text-yellow-400">Level {{ state.currentDifficultyLevel }}</span>
          </div>

          <div class="flex justify-between">
            <span class="text-gray-400">Tick:</span>
            <span class="font-mono text-gray-500">{{ state.tick }}</span>
          </div>
        </div>

        <div v-if="state" class="mt-4 pt-4 border-t border-gray-700">
          <h4 class="text-sm font-semibold mb-2 text-gray-300">Players</h4>
          <div class="space-y-2">
            <div
              v-for="worker in state.workers"
              :key="worker.playerId"
              class="flex items-center gap-2 p-2 rounded"
              :class="{
                'bg-gray-700': worker.playerId === connectionStore.clientId && worker.alive,
                'bg-red-900/50': worker.playerId === connectionStore.clientId && !worker.alive,
                'bg-gray-750': worker.playerId !== connectionStore.clientId,
                'opacity-50': !worker.alive,
              }"
            >
              <div
                class="w-3 h-3 rounded-full"
                :style="{ backgroundColor: getWorkerColor(worker.colorIndex) }"
                :class="{ 'opacity-30': !worker.alive }"
              />
              <span class="text-sm text-gray-200">
                {{ formatPlayerName(worker.playerId, worker.isBot) }}
              </span>
              <span
                v-if="!worker.alive"
                class="text-xs text-red-400 font-semibold"
              >
                DEAD
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.bg-gray-750 {
  background-color: rgb(55 65 81 / 0.5);
}
</style>
