<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from "vue";
import { useGameStore, Direction } from "../../stores/game";
import { useConnectionStore } from "../../stores/connection";
import { useDeviceStore } from "../../stores/device";
import { DeadSnakeBehavior } from "../../proto/games/snake_pb";

const gameStore = useGameStore();
const connectionStore = useConnectionStore();
const deviceStore = useDeviceStore();

const canvasRef = ref<HTMLCanvasElement | null>(null);
const containerRef = ref<HTMLDivElement | null>(null);
const spritesImage = ref<HTMLImageElement | null>(null);
const spritesLoaded = ref(false);
const containerSize = ref({ width: 0, height: 0 });

const SPRITE_SIZE = 64;
const BASE_CELL_SIZE = 48;
const MIN_CELL_SIZE = 24;
const MAX_CELL_SIZE = 80;

const BACKGROUND_COLOR = "#88ff88";
const DEAD_SNAKE_COLOR = { r: 128, g: 128, b: 128 };

interface SpriteCoord {
  col: number;
  row: number;
}

const SPRITES = {
  head_right: { col: 4, row: 0 },
  head_left: { col: 3, row: 1 },
  head_down: { col: 4, row: 1 },
  head_up: { col: 3, row: 0 },
  apple: { col: 0, row: 3 },
  body_horizontal: { col: 1, row: 0 },
  body_vertical: { col: 2, row: 1 },
  tail_left: { col: 3, row: 3 },
  tail_right: { col: 4, row: 2 },
  tail_down: { col: 4, row: 3 },
  tail_up: { col: 3, row: 2 },
  turn_ul: { col: 2, row: 0 },
  turn_ur: { col: 0, row: 0 },
  turn_dl: { col: 2, row: 2 },
  turn_dr: { col: 0, row: 1 },
} as const;

function generateColorFromClientId(clientId: string): { r: number; g: number; b: number } {
  let hash = 0;
  for (let i = 0; i < clientId.length; i++) {
    hash = (Math.imul(hash, 31) + clientId.charCodeAt(i)) >>> 0;
  }

  const hue = hash % 360;
  const saturation = 0.7;
  const lightness = 0.5;

  const c = (1 - Math.abs(2 * lightness - 1)) * saturation;
  const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = lightness - c / 2;

  let r = 0, g = 0, b = 0;
  if (hue < 60) {
    r = c; g = x; b = 0;
  } else if (hue < 120) {
    r = x; g = c; b = 0;
  } else if (hue < 180) {
    r = 0; g = c; b = x;
  } else if (hue < 240) {
    r = 0; g = x; b = c;
  } else if (hue < 300) {
    r = x; g = 0; b = c;
  } else {
    r = c; g = 0; b = x;
  }

  return {
    r: Math.round((r + m) * 255),
    g: Math.round((g + m) * 255),
    b: Math.round((b + m) * 255),
  };
}

const state = computed(() => gameStore.snakeState);

const cellSize = computed(() => {
  if (!state.value || containerSize.value.width === 0) return BASE_CELL_SIZE;

  const availableWidth = containerSize.value.width - 288 - 24;
  const availableHeight = containerSize.value.height - 80;

  const cellByWidth = Math.floor(availableWidth / state.value.fieldWidth);
  const cellByHeight = Math.floor(availableHeight / state.value.fieldHeight);

  const size = Math.min(cellByWidth, cellByHeight);
  return Math.max(MIN_CELL_SIZE, Math.min(MAX_CELL_SIZE, size));
});

const canvasWidth = computed(() => {
  if (!state.value) return 400;
  return state.value.fieldWidth * cellSize.value;
});

const canvasHeight = computed(() => {
  if (!state.value) return 400;
  return state.value.fieldHeight * cellSize.value;
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

const showDeadSnakes = computed(() => {
  if (!state.value) return false;
  if (gameStore.isGameOver) return true;
  return state.value.deadSnakeBehavior === DeadSnakeBehavior.STAY_ON_FIELD;
});

const snakeColors = computed(() => {
  const colors: Map<string, { r: number; g: number; b: number }> = new Map();
  if (!state.value) return colors;

  for (const snake of state.value.snakes) {
    const playerId = snake.identity?.playerId;
    if (playerId) {
      colors.set(playerId, generateColorFromClientId(playerId));
    }
  }
  return colors;
});

function getSnakeColor(playerId: string | undefined, alive: boolean): { r: number; g: number; b: number } {
  if (!alive) return DEAD_SNAKE_COLOR;
  if (!playerId) return { r: 34, g: 197, b: 94 };
  return snakeColors.value.get(playerId) ?? generateColorFromClientId(playerId);
}

function getSnakeColorHex(playerId: string | undefined, alive: boolean): string {
  const { r, g, b } = getSnakeColor(playerId, alive);
  return `rgb(${r}, ${g}, ${b})`;
}

function lessModulo(a: number, b: number, modulo: number): boolean {
  return b === (a + 1) % modulo;
}

function greaterModulo(a: number, b: number, modulo: number): boolean {
  return a === (b + 1) % modulo;
}

function getHeadDirection(
  headX: number, headY: number,
  nextX: number, nextY: number,
  fieldWidth: number, fieldHeight: number
): Direction {
  if (lessModulo(headY, nextY, fieldHeight)) return Direction.UP;
  if (greaterModulo(headY, nextY, fieldHeight)) return Direction.DOWN;
  if (lessModulo(headX, nextX, fieldWidth)) return Direction.LEFT;
  return Direction.RIGHT;
}

function getHeadSprite(direction: Direction): SpriteCoord {
  switch (direction) {
    case Direction.UP: return SPRITES.head_up;
    case Direction.DOWN: return SPRITES.head_down;
    case Direction.LEFT: return SPRITES.head_left;
    case Direction.RIGHT: return SPRITES.head_right;
    default: return SPRITES.head_up;
  }
}

function getTailSprite(
  fromX: number, fromY: number,
  toX: number, toY: number,
  fieldWidth: number, fieldHeight: number
): SpriteCoord {
  if (lessModulo(fromY, toY, fieldHeight)) {
    return SPRITES.tail_up;
  } else if (greaterModulo(fromY, toY, fieldHeight)) {
    return SPRITES.tail_down;
  } else if (lessModulo(fromX, toX, fieldWidth)) {
    return SPRITES.tail_left;
  } else {
    return SPRITES.tail_right;
  }
}

function getBodySprite(
  prevX: number, prevY: number,
  currX: number, currY: number,
  nextX: number, nextY: number,
  fieldWidth: number, fieldHeight: number
): SpriteCoord {
  const prevLeft = lessModulo(prevX, currX, fieldWidth);
  const prevRight = greaterModulo(prevX, currX, fieldWidth);
  const prevUp = lessModulo(prevY, currY, fieldHeight);
  const prevDown = greaterModulo(prevY, currY, fieldHeight);

  const nextLeft = lessModulo(nextX, currX, fieldWidth);
  const nextRight = greaterModulo(nextX, currX, fieldWidth);
  const nextUp = lessModulo(nextY, currY, fieldHeight);
  const nextDown = greaterModulo(nextY, currY, fieldHeight);

  if ((prevLeft && nextRight) || (prevRight && nextLeft)) {
    return SPRITES.body_horizontal;
  } else if ((prevUp && nextDown) || (prevDown && nextUp)) {
    return SPRITES.body_vertical;
  } else if ((prevUp && nextLeft) || (prevLeft && nextUp)) {
    return SPRITES.turn_dl;
  } else if ((prevUp && nextRight) || (prevRight && nextUp)) {
    return SPRITES.turn_dr;
  } else if ((prevDown && nextLeft) || (prevLeft && nextDown)) {
    return SPRITES.turn_ul;
  } else {
    return SPRITES.turn_ur;
  }
}

function drawSpriteTinted(
  ctx: CanvasRenderingContext2D,
  sprite: SpriteCoord,
  x: number, y: number,
  color: { r: number; g: number; b: number }
): void {
  if (!spritesImage.value) return;

  const offscreen = document.createElement("canvas");
  offscreen.width = SPRITE_SIZE;
  offscreen.height = SPRITE_SIZE;
  const offCtx = offscreen.getContext("2d");
  if (!offCtx) return;

  offCtx.drawImage(
    spritesImage.value,
    sprite.col * SPRITE_SIZE,
    sprite.row * SPRITE_SIZE,
    SPRITE_SIZE,
    SPRITE_SIZE,
    0, 0,
    SPRITE_SIZE, SPRITE_SIZE
  );

  const imageData = offCtx.getImageData(0, 0, SPRITE_SIZE, SPRITE_SIZE);
  const data = imageData.data;

  for (let i = 0; i < data.length; i += 4) {
    const alpha = data[i + 3]!;
    if (alpha > 0) {
      const gray = (data[i]! + data[i + 1]! + data[i + 2]!) / 3 / 255;
      data[i] = Math.round(color.r * gray);
      data[i + 1] = Math.round(color.g * gray);
      data[i + 2] = Math.round(color.b * gray);
    }
  }

  offCtx.putImageData(imageData, 0, 0);

  ctx.drawImage(offscreen, x, y, cellSize.value, cellSize.value);
}

function drawSpriteUntinted(
  ctx: CanvasRenderingContext2D,
  sprite: SpriteCoord,
  x: number, y: number
): void {
  if (!spritesImage.value) return;

  ctx.drawImage(
    spritesImage.value,
    sprite.col * SPRITE_SIZE,
    sprite.row * SPRITE_SIZE,
    SPRITE_SIZE,
    SPRITE_SIZE,
    x, y,
    cellSize.value, cellSize.value
  );
}

function draw(): void {
  const canvas = canvasRef.value;
  const ctx = canvas?.getContext("2d");
  if (!canvas || !ctx || !state.value) return;

  ctx.fillStyle = BACKGROUND_COLOR;
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  if (!spritesLoaded.value) {
    drawFallback(ctx);
    return;
  }

  drawFood(ctx);
  drawSnakes(ctx);
}

function drawFallback(ctx: CanvasRenderingContext2D): void {
  if (!state.value) return;
  const size = cellSize.value;

  ctx.fillStyle = "#ef4444";
  for (const food of state.value.food) {
    const x = food.x * size + size / 2;
    const y = food.y * size + size / 2;
    ctx.beginPath();
    ctx.arc(x, y, size / 2 - 2, 0, Math.PI * 2);
    ctx.fill();
  }

  for (const snake of state.value.snakes) {
    if (!snake.alive && !showDeadSnakes.value) continue;

    const color = getSnakeColorHex(snake.identity?.playerId, snake.alive);
    ctx.fillStyle = color;
    for (const segment of snake.segments) {
      ctx.fillRect(
        segment.x * size + 2,
        segment.y * size + 2,
        size - 4,
        size - 4
      );
    }
  }
}

function drawFood(ctx: CanvasRenderingContext2D): void {
  if (!state.value) return;
  const size = cellSize.value;

  for (const food of state.value.food) {
    drawSpriteUntinted(ctx, SPRITES.apple, food.x * size, food.y * size);
  }
}

function drawSnakes(ctx: CanvasRenderingContext2D): void {
  if (!state.value) return;

  const fieldWidth = state.value.fieldWidth;
  const fieldHeight = state.value.fieldHeight;
  const size = cellSize.value;

  for (const snake of state.value.snakes) {
    if (!snake.alive && !showDeadSnakes.value) continue;

    const color = getSnakeColor(snake.identity?.playerId, snake.alive);
    const segments = snake.segments;

    if (segments.length === 0) continue;

    for (let i = 0; i < segments.length; i++) {
      const segment = segments[i];
      if (!segment) continue;

      const x = segment.x * size;
      const y = segment.y * size;

      let sprite: SpriteCoord;

      if (i === 0) {
        const next = segments[1];
        if (next) {
          const direction = getHeadDirection(segment.x, segment.y, next.x, next.y, fieldWidth, fieldHeight);
          sprite = getHeadSprite(direction);
        } else {
          sprite = SPRITES.head_up;
        }
      } else if (i === segments.length - 1) {
        const prev = segments[i - 1];
        if (!prev) continue;
        sprite = getTailSprite(prev.x, prev.y, segment.x, segment.y, fieldWidth, fieldHeight);
      } else {
        const prev = segments[i - 1];
        const next = segments[i + 1];
        if (!prev || !next) continue;
        sprite = getBodySprite(
          prev.x, prev.y,
          segment.x, segment.y,
          next.x, next.y,
          fieldWidth, fieldHeight
        );
      }

      drawSpriteTinted(ctx, sprite, x, y, color);
    }
  }
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
  if (!isAlive.value) return;

  const touch = event.changedTouches[0];
  if (!touch) return;

  const deltaX = touch.clientX - touchStartX;
  const deltaY = touch.clientY - touchStartY;

  const absX = Math.abs(deltaX);
  const absY = Math.abs(deltaY);

  if (absX < SWIPE_THRESHOLD && absY < SWIPE_THRESHOLD) return;

  let direction: Direction;
  if (absX > absY) {
    direction = deltaX > 0 ? Direction.RIGHT : Direction.LEFT;
  } else {
    direction = deltaY > 0 ? Direction.DOWN : Direction.UP;
  }

  gameStore.sendSnakeCommand(direction);
}

function formatPlayerName(playerId: string, isBot: boolean): string {
  if (isBot) return `${playerId} [BOT]`;
  return playerId;
}

function loadSprites(): void {
  const img = new Image();
  img.onload = () => {
    spritesImage.value = img;
    spritesLoaded.value = true;
    draw();
  };
  img.onerror = () => {
    console.warn("Failed to load sprites, using fallback rendering");
    spritesLoaded.value = false;
  };
  img.src = import.meta.env.BASE_URL + "sprites.png";
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

watch(state, () => {
  draw();
});

watch(cellSize, () => {
  draw();
});

onMounted(() => {
  window.addEventListener("keydown", handleKeyDown);
  loadSprites();

  updateContainerSize();
  resizeObserver = new ResizeObserver(() => {
    updateContainerSize();
    draw();
  });
  if (containerRef.value) {
    resizeObserver.observe(containerRef.value);
  }

  draw();
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeyDown);
  if (resizeObserver) {
    resizeObserver.disconnect();
    resizeObserver = null;
  }
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
          class="block touch-none"
          @touchstart="handleTouchStart"
          @touchend="handleTouchEnd"
        />
      </div>

      <div class="mt-4 text-center text-gray-400 text-sm">
        <template v-if="isAlive">
          <template v-if="deviceStore.isTouchDevice">
            Swipe to change direction
          </template>
          <template v-else>
            Use <span class="font-mono bg-gray-700 px-1 rounded">Arrow Keys</span>
            or
            <span class="font-mono bg-gray-700 px-1 rounded">WASD</span>
            to move
          </template>
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
            v-for="snake in sortedSnakes"
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
                :style="{ backgroundColor: getSnakeColorHex(snake.identity?.playerId, snake.alive) }"
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
