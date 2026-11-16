<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, shallowRef, watch } from "vue";

import BotIcon from "@/components/BotIcon.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import { useGameKeys } from "@/input/use-game-keys";
import type { MatchController } from "@/match/match-controller";
import type { WordHuntLetterMark } from "@/proto/minigames/v2/word_hunt_pb";
import { WordHuntPhase, WordHuntPuzzleMode } from "@/proto/minigames/v2/word_hunt_pb";
import { prefersReducedMotion } from "@/render/reduced-motion";
import { seatLabels } from "@/util/players";

import { formatDay } from "./daily";
import { keyboardRows, letterForKey } from "./keyboard-layout";
import { markClass } from "./mark-style";
import { marksMissingLetters } from "./preferences";
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
const header = props.controller.source.header;
/** Two or more seats take turns on this board. */
const isGroup = header.seats.length > 1;
const names = seatLabels(header);
const myTurn = computed(
  () => props.controller.mySlot !== null && view.value?.turnSlot === props.controller.mySlot,
);
const canType = computed(() => isPlaying.value && myTurn.value);

/** Seat colours mark who made each guess; none of them is a hint colour. */
const SEAT_COLORS = ["bg-sky-500", "bg-rose-500", "bg-violet-500", "bg-teal-500"] as const;
function seatColor(slot: number): string {
  return SEAT_COLORS[slot % SEAT_COLORS.length] ?? "bg-gray-500";
}
function seatName(slot: number): string {
  return names[slot] ?? `Player ${slot + 1}`;
}
function isBotSeat(slot: number): boolean {
  return header.seats.find((seat) => seat.slot === slot)?.botKind !== undefined;
}

const turnLine = computed(() => {
  const turn = view.value?.turnSlot;
  if (!isGroup || !isPlaying.value || turn === undefined) return null;
  if (gaveUp.value) return "You gave up · watching";
  if (myTurn.value) return "Your turn";
  return isBotSeat(turn) ? `${seatName(turn)} is thinking…` : `${seatName(turn)}'s turn`;
});

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

/** The last letter sends the guess: there is no Enter to press. */
function type(letter: string): void {
  if (!canType.value || typed.value.length >= wordLength.value) return;
  typed.value += letter;
  if (typed.value.length === wordLength.value) void enter();
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

/** In a group the seat drops out and the game goes on, so the button must go away. */
const gaveUp = ref(false);
const confirmingGiveUp = ref(false);
const canGiveUp = computed(() => props.controller.mySlot !== null && isPlaying.value && !gaveUp.value);
const giveUpMessage = isGroup
  ? "You drop out and the others play on. The word is shown when the game ends."
  : "The game ends as a loss and the word is shown.";

async function giveUp(): Promise<void> {
  confirmingGiveUp.value = false;
  try {
    await props.controller.submit({
      game: { case: "wordHunt", value: { command: { case: "giveUp", value: {} } } },
    });
    gaveUp.value = true;
  } catch (error) {
    refuse(refusalText(error));
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
  /** Group games: who made the guess. */
  readonly by: number | null;
}

/**
 * Alone: a fixed grid of the guesses allowed. In a group there is no fixed number: the guesses
 * so far and a row for the next one, scrolled to the newest.
 */
const rows = computed<TileRow[]>(() => {
  const result: TileRow[] = guesses.value.map((guess) => ({
    letters: [...guess.word],
    marks: guess.marks,
    current: false,
    by: isGroup ? guess.bySlot : null,
  }));
  const limit = view.value?.guessLimit ?? maxGuesses.value;
  if (isPlaying.value && result.length < limit) {
    result.push({
      letters: canType.value ? [...typed.value] : [],
      marks: null,
      current: true,
      by: isGroup ? (view.value?.turnSlot ?? null) : null,
    });
  }
  if (!isGroup) {
    while (result.length < maxGuesses.value)
      result.push({ letters: [], marks: null, current: false, by: null });
  }
  return result;
});

const scroller = ref<HTMLElement | null>(null);
watch(
  () => guesses.value.length,
  async () => {
    await nextTick();
    scroller.value?.scrollTo({ top: scroller.value.scrollHeight, behavior: "smooth" });
  },
);

function tileClass(row: TileRow, column: number): string {
  const mark = row.marks?.[column];
  if (mark !== undefined) return markClass(mark);
  return row.letters[column] ? "border-2 border-gray-400 text-gray-100" : "border-2 border-white/10";
}

/** After a miss the answer gets its own row of tiles under the guesses. */
const missedAnswer = computed(() => {
  const current = view.value;
  return current?.answer && current.phase !== WordHuntPhase.SOLVED ? [...current.answer] : null;
});

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
  // Stats are about solving alone; a group game is won or lost against the others.
  if (!current || mySlot === null || source.kind === "replay" || isGroup) return null;
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
  () =>
    stats.value ??
    (settings.value && !isPlaying.value && !isGroup ? statsFor(settings.value.language) : null),
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
      <p v-if="turnLine" class="text-sm font-semibold" :class="myTurn ? 'text-emerald-300' : 'text-gray-300'">
        {{ turnLine }}
      </p>
      <div class="wh-grid relative flex flex-col gap-1.5" :style="gridSize">
        <div
          ref="scroller"
          class="flex flex-col gap-1.5"
          :class="isGroup ? 'wh-scroll overflow-y-auto overscroll-contain px-1' : ''"
        >
          <div
            v-for="(row, rowIndex) in rows"
            :key="`${rowIndex}-${row.current ? shakes : 0}`"
            class="flex items-center gap-1.5"
            :class="{ 'wh-shake': row.current && shakes > 0 && message !== null && !prefersReducedMotion() }"
          >
            <span
              v-if="isGroup"
              class="flex size-6 shrink-0 items-center justify-center rounded-full text-xs font-bold text-white"
              :class="row.by === null ? 'invisible' : seatColor(row.by)"
              :title="row.by === null ? undefined : seatName(row.by)"
            >
              <BotIcon v-if="row.by !== null && isBotSeat(row.by)" class="size-4" />
              <template v-else>{{
                row.by === null ? "" : seatName(row.by).charAt(0).toUpperCase()
              }}</template>
            </span>
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
        </div>
        <div v-if="missedAnswer" class="mt-3 flex flex-col items-center gap-1.5">
          <p class="text-xs font-semibold uppercase tracking-widest text-gray-400">The word was</p>
          <div class="flex gap-1.5">
            <div
              v-for="(letter, index) in missedAnswer"
              :key="index"
              class="flex size-(--tile) items-center justify-center rounded-lg bg-emerald-600 text-[calc(var(--tile)*0.5)] font-bold uppercase text-white shadow-lg shadow-emerald-900/50"
              :class="prefersReducedMotion() ? '' : 'wh-reveal'"
              :style="{ animationDelay: `${600 + index * 90}ms` }"
            >
              {{ letter }}
            </div>
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
    <button
      v-if="canGiveUp"
      type="button"
      class="text-xs text-gray-400 underline-offset-4 hover:text-gray-200 hover:underline"
      @click="confirmingGiveUp = true"
    >
      Give up
    </button>
    <ConfirmDialog
      v-if="confirmingGiveUp"
      title="Give up?"
      :message="giveUpMessage"
      confirm-label="Give up"
      cancel-label="Keep guessing"
      @confirm="giveUp"
      @cancel="confirmingGiveUp = false"
    />
    <WordHuntKeyboard
      v-if="controller.mySlot !== null"
      class="wh-keyboard mt-auto"
      :rows="keyboardRows(settings.language)"
      :hints="view.keyboard"
      :disabled="!canType"
      :marks-missing="marksMissingLetters"
      @letter="type"
      @erase="erase"
    />
  </div>
</template>

<style scoped>
/* Tiles fill the width on phones and the height on short screens, between a floor that keeps
   letters readable and a cap for tablets. */
/* A group's guesses scroll within about six rows, so the keyboard stays in place. */
.wh-scroll {
  max-height: calc(6 * (var(--tile) + 0.375rem));
}
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
