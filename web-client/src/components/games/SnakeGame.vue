<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from "vue";
import { useGameStore, Direction } from "../../stores/game";
import { useConnectionStore } from "../../stores/connection";

const gameStore = useGameStore();
const connectionStore = useConnectionStore();

const canvasRef = ref<HTMLCanvasElement | null>(null);
const containerRef = ref<HTMLDivElement | null>(null);

const CELL_SIZE = 20;
const GRID_LINE_WIDTH = 1;

const SNAKE_COLORS = [
  "#22c55e", // green
  "#3b82f6", // blue
  "#f59e0b", // amber
  "#ef4444", // red
  "#8b5cf6", // violet
  "#ec4899", // pink
  "#14b8a6", // teal
  "#f97316", // orange
];

const DEAD_SNAKE_COLOR = "#4b5563";
const FOOD_COLOR = "#ef4444";
const GRID_COLOR = "#374151";
const BACKGROUND_COLOR = "#1f2937";

const state = computed(() => gameStore.snakeState);

const canvasWidth = computed(() => {
  if (!state.value) return 400;
  return state.value.fieldWidth * CELL_SIZE;
});

const canvasHeight = computed(() => {
  if (!state.value) return 400;
  return state.value.fieldHeight * CELL_SIZE;
});

const sortedSnakes = computed(() => {
  if (!state.value) return [];
  return [...state.value.snakes].sort((a, b) => b.score - a.score);
});

const mySnake = computed(() => {
  if (!state.value || !connectionStore.clientId) return null;
  return state.value.snakes.find(
    (s) => s.identity?.playerId === connectionStore.clientId
  );
});

const isAlive = computed(() => mySnake.value?.alive ?? false);

function getSnakeColor(index: number, alive: boolean): string {
  if (!alive) return DEAD_SNAKE_COLOR;
  return SNAKE_COLORS[index % SNAKE_COLORS.length] ?? SNAKE_COLORS[0] ?? "#22c55e";
}

function draw(): void {
  const canvas = canvasRef.value;
  const ctx = canvas?.getContext("2d");
  if (!canvas || !ctx || !state.value) return;

  ctx.fillStyle = BACKGROUND_COLOR;
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  drawGrid(ctx);
  drawFood(ctx);
  drawSnakes(ctx);
}

function drawGrid(ctx: CanvasRenderingContext2D): void {
  if (!state.value) return;

  ctx.strokeStyle = GRID_COLOR;
  ctx.lineWidth = GRID_LINE_WIDTH;

  for (let x = 0; x <= state.value.fieldWidth; x++) {
    ctx.beginPath();
    ctx.moveTo(x * CELL_SIZE, 0);
    ctx.lineTo(x * CELL_SIZE, canvasHeight.value);
    ctx.stroke();
  }

  for (let y = 0; y <= state.value.fieldHeight; y++) {
    ctx.beginPath();
    ctx.moveTo(0, y * CELL_SIZE);
    ctx.lineTo(canvasWidth.value, y * CELL_SIZE);
    ctx.stroke();
  }
}

function drawFood(ctx: CanvasRenderingContext2D): void {
  if (!state.value) return;

  ctx.fillStyle = FOOD_COLOR;
  for (const food of state.value.food) {
    const x = food.x * CELL_SIZE + CELL_SIZE / 2;
    const y = food.y * CELL_SIZE + CELL_SIZE / 2;
    const radius = CELL_SIZE / 2 - 2;

    ctx.beginPath();
    ctx.arc(x, y, radius, 0, Math.PI * 2);
    ctx.fill();
  }
}

function drawSnakes(ctx: CanvasRenderingContext2D): void {
  if (!state.value) return;

  state.value.snakes.forEach((snake, index) => {
    const color = getSnakeColor(index, snake.alive);
    const isCurrentPlayer = snake.identity?.playerId === connectionStore.clientId;

    for (let i = 0; i < snake.segments.length; i++) {
      const segment = snake.segments[i];
      if (!segment) continue;
      const isHead = i === 0;
      const x = segment.x * CELL_SIZE;
      const y = segment.y * CELL_SIZE;

      ctx.fillStyle = color;

      if (isHead) {
        ctx.beginPath();
        ctx.roundRect(x + 1, y + 1, CELL_SIZE - 2, CELL_SIZE - 2, 4);
        ctx.fill();

        if (isCurrentPlayer && snake.alive) {
          ctx.strokeStyle = "#ffffff";
          ctx.lineWidth = 2;
          ctx.stroke();
        }
      } else {
        ctx.beginPath();
        ctx.roundRect(x + 2, y + 2, CELL_SIZE - 4, CELL_SIZE - 4, 2);
        ctx.fill();
      }
    }
  });
}

function handleKeyDown(event: KeyboardEvent): void {
  if (!isAlive.value) return;

  let direction: Direction | null = null;

  switch (event.key) {
    case "ArrowUp":
    case "w":
    case "W":
      direction = Direction.UP;
      break;
    case "ArrowDown":
    case "s":
    case "S":
      direction = Direction.DOWN;
      break;
    case "ArrowLeft":
    case "a":
    case "A":
      direction = Direction.LEFT;
      break;
    case "ArrowRight":
    case "d":
    case "D":
      direction = Direction.RIGHT;
      break;
  }

  if (direction !== null) {
    event.preventDefault();
    gameStore.sendSnakeCommand(direction);
  }
}

function formatPlayerName(playerId: string, isBot: boolean): string {
  if (isBot) return `${playerId} [BOT]`;
  return playerId;
}

watch(state, () => {
  draw();
});

onMounted(() => {
  window.addEventListener("keydown", handleKeyDown);
  draw();
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeyDown);
});
</script>

<template>
  <div ref="containerRef" class="flex flex-col lg:flex-row gap-6">
    <div class="flex-shrink-0">
      <div
        class="border-2 border-gray-600 rounded-lg overflow-hidden"
        :style="{ width: canvasWidth + 'px' }"
      >
        <canvas
          ref="canvasRef"
          :width="canvasWidth"
          :height="canvasHeight"
          class="block"
        />
      </div>

      <div class="mt-4 text-center text-gray-400 text-sm">
        <template v-if="isAlive">
          Use <span class="font-mono bg-gray-700 px-1 rounded">Arrow Keys</span>
          or
          <span class="font-mono bg-gray-700 px-1 rounded">WASD</span>
          to move
        </template>
        <template v-else>
          <span class="text-red-400">You have been eliminated</span>
        </template>
      </div>
    </div>

    <div class="flex-grow min-w-64">
      <div class="bg-gray-800 rounded-lg p-4">
        <h3 class="text-lg font-semibold mb-4 text-gray-200">Scoreboard</h3>

        <div class="space-y-2">
          <div
            v-for="(snake, index) in sortedSnakes"
            :key="snake.identity?.playerId"
            class="flex items-center justify-between p-2 rounded"
            :class="{
              'bg-gray-700': snake.identity?.playerId === connectionStore.clientId,
              'bg-gray-750': snake.identity?.playerId !== connectionStore.clientId,
            }"
          >
            <div class="flex items-center gap-3">
              <div
                class="w-4 h-4 rounded"
                :style="{ backgroundColor: getSnakeColor(index, snake.alive) }"
              />
              <span
                class="font-medium"
                :class="{
                  'text-gray-200': snake.alive,
                  'text-gray-500 line-through': !snake.alive,
                }"
              >
                {{ formatPlayerName(snake.identity?.playerId ?? "Unknown", snake.identity?.isBot ?? false) }}
              </span>
            </div>
            <span
              class="font-bold text-lg"
              :class="{
                'text-green-400': snake.alive,
                'text-gray-500': !snake.alive,
              }"
            >
              {{ snake.score }}
            </span>
          </div>
        </div>

        <div v-if="state" class="mt-4 pt-4 border-t border-gray-700">
          <div class="text-sm text-gray-400">
            <div class="flex justify-between">
              <span>Tick:</span>
              <span class="font-mono">{{ state.tick }}</span>
            </div>
            <div class="flex justify-between">
              <span>Food on field:</span>
              <span class="font-mono">{{ state.food.length }}</span>
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
