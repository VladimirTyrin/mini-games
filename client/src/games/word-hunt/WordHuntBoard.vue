<script setup lang="ts">
import { computed, onBeforeUnmount, ref, shallowRef, watch } from "vue";

import { useGameKeys } from "@/input/use-game-keys";
import type { MatchController } from "@/match/match-controller";
import type { WordHuntLetterMark } from "@/proto/minigames/v2/word_hunt_pb";
import { WordHuntPhase, WordHuntPuzzleMode } from "@/proto/minigames/v2/word_hunt_pb";
import { prefersReducedMotion } from "@/render/reduced-motion";

import { formatDay } from "./daily";
import { keyboardRows, letterForKey } from "./keyboard-layout";
import { markClass } from "./mark-style";
import { type StatsGame, type WordHuntStats, finishGame, startGame, statsFor } from "./stats";
import WordHuntKeyboard from "./WordHuntKeyboard.vue";
import { type AcceptedWords, acceptedWords, normalizeWord } from "./word-lists";

const props = defineProps<{ controller: MatchController }>();

const MESSAGE_MS = 2200;

const view = computed(() => {
  const game = props.controller.current.value?.view.game;
  return game?.case === "wordHunt" ? game.value : null;
});
const settings = computed(() => view.value?.settings ?? null);
const wordLength = computed(() => settings.value?.wordLength ?? 5);
const maxGuesses = computed(() => settings.value?.maxGuesses ?? 6);
const guesses = computed(() => view.value?.guesses ?? []);
const isPlaying = computed(() => view.value?.phase === WordHuntPhase.GUESSING);
const canType = computed(() => props.controller.mySlot !== null && isPlaying.value);

const typed = ref("");
const submitting = ref(false);
watch(
  () => guesses.value.length,
  () => {
    typed.value = "";
  },
);

const words = shallowRef<AcceptedWords | null>(null);
watch(
  () => settings.value?.language,
  (language) => {
    words.value = null;
    if (language === undefined) return;
    acceptedWords(language)
      .then((loaded) => {
        if (settings.value?.language === language) words.value = loaded;
      })
      .catch(() => {
        // Offline before the list was ever cached: guesses go unchecked rather than blocked.
      });
  },
  { immediate: true },
);

const message = ref<string | null>(null);
/** Bumped to restart the shake when the same guess is refused again. */
const shakes = ref(0);
let messageTimer: ReturnType<typeof setTimeout> | undefined;
function refuse(text: string): void {
  message.value = text;
  shakes.value += 1;
  clearTimeout(messageTimer);
  messageTimer = setTimeout(() => (message.value = null), MESSAGE_MS);
}
onBeforeUnmount(() => clearTimeout(messageTimer));

/** The core's refusal ("illegal move: hard mode: …") as a sentence for the player. */
function refusalText(error: unknown): string {
  const text = (error instanceof Error ? error.message : String(error)).replace(/^illegal move: /, "");
  return text.charAt(0).toUpperCase() + text.slice(1);
}

function type(letter: string): void {
  if (canType.value && typed.value.length < wordLength.value) typed.value += letter;
}

function erase(): void {
  if (canType.value) typed.value = typed.value.slice(0, -1);
}

async function enter(): Promise<void> {
  if (!canType.value || submitting.value) return;
  const guess = normalizeWord(typed.value);
  if (guess.length < wordLength.value) return refuse("Not enough letters");
  const known = words.value?.get(wordLength.value);
  if (known && !known.has(guess)) return refuse("Not in the word list");
  submitting.value = true;
  try {
    await props.controller.submit({
      game: { case: "wordHunt", value: { command: { case: "guess", value: { word: guess } } } },
    });
  } catch (error) {
    refuse(refusalText(error));
  } finally {
    submitting.value = false;
  }
}

// Repeats on: holding Backspace or a letter keeps going, like any text field. Enter acts once.
useGameKeys(
  (key, event) => {
    if (!canType.value || !settings.value) return false;
    if (key === "Enter") {
      if (!event.repeat) void enter();
      return true;
    }
    if (key === "Backspace") {
      erase();
      return true;
    }
    const letter = letterForKey(settings.value.language, key, event.code);
    if (letter === null) return false;
    type(letter);
    return true;
  },
  { allowRepeat: true },
);

interface TileRow {
  readonly letters: readonly string[];
  readonly marks: readonly WordHuntLetterMark[] | null;
  readonly current: boolean;
}

const rows = computed<TileRow[]>(() => {
  const result: TileRow[] = guesses.value.map((guess) => ({
    letters: [...guess.word],
    marks: guess.marks,
    current: false,
  }));
  if (isPlaying.value && result.length < maxGuesses.value) {
    result.push({ letters: [...typed.value], marks: null, current: true });
  }
  while (result.length < maxGuesses.value) result.push({ letters: [], marks: null, current: false });
  return result;
});

function tileClass(row: TileRow, column: number): string {
  const mark = row.marks?.[column];
  if (mark !== undefined) return markClass(mark);
  return row.letters[column] ? "border-2 border-gray-400 text-gray-100" : "border-2 border-white/10";
}

/** The newest guess reveals tile by tile. */
const revealRow = computed(() => (prefersReducedMotion() ? -1 : guesses.value.length - 1));

const subtitle = computed(() => {
  const current = settings.value;
  if (!current) return "";
  const kind =
    current.mode === WordHuntPuzzleMode.DAILY
      ? `Word of the day · ${formatDay(current.day)}`
      : `${current.wordLength} letters`;
  return current.hardMode ? `${kind} · hard mode` : kind;
});

/** The player's own games only: never replays or observed matches. */
const statsGame = computed<StatsGame | null>(() => {
  const current = settings.value;
  const { source, mySlot } = props.controller;
  if (!current || mySlot === null || source.kind === "replay") return null;
  return {
    language: current.language,
    key: `${source.header.matchId}:${source.header.startedAtUnixMs}`,
    dailyDay: current.mode === WordHuntPuzzleMode.DAILY ? current.day : null,
  };
});
const stats = ref<WordHuntStats | null>(null);
watch(
  [() => guesses.value.length, () => view.value?.phase],
  ([guessCount, phase]) => {
    const game = statsGame.value;
    if (!game || guessCount === 0) return;
    startGame(game);
    if (phase === WordHuntPhase.SOLVED) stats.value = finishGame(game, guessCount);
    else if (phase === WordHuntPhase.FAILED) stats.value = finishGame(game, null);
  },
  { immediate: true },
);
const shownStats = computed(
  () => stats.value ?? (settings.value && !isPlaying.value ? statsFor(settings.value.language) : null),
);
const winRate = computed(() => {
  const current = shownStats.value;
  return current && current.played > 0 ? Math.round((100 * current.won) / current.played) : 0;
});

const gridSize = computed(() => ({ "--letters": wordLength.value, "--rows": maxGuesses.value }));
</script>

<template>
  <div v-if="view && settings" class="wh-layout flex min-h-0 flex-1 flex-col items-center gap-3">
    <div class="flex flex-col items-center gap-3">
      <p class="text-sm text-gray-400">{{ subtitle }}</p>
      <div class="wh-grid relative flex flex-col gap-1.5" :style="gridSize">
        <div
          v-for="(row, rowIndex) in rows"
          :key="`${rowIndex}-${row.current ? shakes : 0}`"
          class="flex gap-1.5"
          :class="{ 'wh-shake': row.current && shakes > 0 && message !== null && !prefersReducedMotion() }"
        >
          <div
            v-for="column in wordLength"
            :key="column"
            class="flex size-(--tile) items-center justify-center rounded-lg text-[calc(var(--tile)*0.5)] font-bold uppercase"
            :class="[tileClass(row, column - 1), rowIndex === revealRow ? 'wh-reveal' : '']"
            :style="rowIndex === revealRow ? { animationDelay: `${(column - 1) * 90}ms` } : undefined"
          >
            {{ row.letters[column - 1] ?? "" }}
          </div>
        </div>
        <p
          v-if="message"
          role="alert"
          class="absolute left-1/2 top-1/3 -translate-x-1/2 whitespace-nowrap rounded-lg bg-gray-100 px-3 py-2 text-sm font-semibold text-gray-900 shadow-lg"
        >
          {{ message }}
        </p>
      </div>
      <p v-if="shownStats && !isPlaying" class="text-sm text-gray-400">
        Played {{ shownStats.played }} · won {{ winRate }}% · streak {{ shownStats.streak }} (best
        {{ shownStats.bestStreak }})
      </p>
    </div>
    <WordHuntKeyboard
      v-if="controller.mySlot !== null"
      class="wh-keyboard mt-auto"
      :rows="keyboardRows(settings.language)"
      :hints="view.keyboard"
      :disabled="!canType"
      @letter="type"
      @erase="erase"
      @enter="enter"
    />
  </div>
</template>

<style scoped>
/* Tiles fill the width on phones and the height on short screens, between a floor that keeps
   letters readable and a cap for tablets. */
.wh-grid {
  --tile: max(
    1.75rem,
    min(
      4.5rem,
      calc((100vw - 2rem) / var(--letters) - 0.375rem),
      calc((100dvh - 20rem) / var(--rows) - 0.375rem)
    )
  );
}
/* A phone held sideways: the keyboard goes beside the board. */
@media (orientation: landscape) and (max-height: 520px) {
  .wh-layout {
    flex-direction: row;
    justify-content: center;
  }
  .wh-keyboard {
    margin-top: 0;
    max-width: 50vw;
  }
  .wh-grid {
    --tile: max(
      1.5rem,
      min(
        3rem,
        calc((100dvh - 9rem) / var(--rows) - 0.375rem),
        calc((45vw - 2rem) / var(--letters) - 0.375rem)
      )
    );
  }
}
.wh-shake {
  animation: wh-shake 360ms ease-in-out;
}
@keyframes wh-shake {
  20%,
  60% {
    transform: translateX(-6px);
  }
  40%,
  80% {
    transform: translateX(6px);
  }
}
.wh-reveal {
  animation: wh-reveal 420ms ease-out both;
}
@keyframes wh-reveal {
  from {
    transform: scale(0.6);
    opacity: 0.2;
  }
  60% {
    transform: scale(1.08);
  }
  to {
    transform: scale(1);
    opacity: 1;
  }
}
</style>
