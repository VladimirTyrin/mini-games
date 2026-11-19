<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, shallowRef, watch } from "vue";

import Avatar from "@/components/Avatar.vue";
import BotIcon from "@/components/BotIcon.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import { t } from "@/i18n";
import { useGameKeys } from "@/input/use-game-keys";
import type { MatchController } from "@/match/match-controller";
import { OutcomeKind } from "@/proto/minigames/v2/match_pb";
import type { WordHuntLetterMark } from "@/proto/minigames/v2/word_hunt_pb";
import { WordHuntPhase, WordHuntPuzzleMode } from "@/proto/minigames/v2/word_hunt_pb";
import { prefersReducedMotion } from "@/render/reduced-motion";
import { matchSeatNames } from "@/games/seat-names";
import { errorMessage } from "@/util/error-message";
import { refusalText } from "@/util/refusal-text";

import { formatDay } from "./daily";
import { keyboardRows, letterForKey } from "./keyboard-layout";
import { inPlaceClass, markClass } from "./mark-style";
import { marksMissingLetters } from "./preferences";
import { type StatsGame, type StatsProgress, type WordHuntStats, recordProgress, statsFor } from "./stats";
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
const names = matchSeatNames(header);
const myTurn = computed(
  () => props.controller.mySlot !== null && view.value?.turnSlot === props.controller.mySlot,
);
const canType = computed(() => isPlaying.value && myTurn.value);

/** Seat colours mark who made each guess; none of them is a hint colour. */
const SEAT_COLORS = ["bg-sky-500", "bg-rose-500", "bg-violet-500", "bg-teal-500"] as const;
function seatColor(slot: number): string {
  return SEAT_COLORS[slot % SEAT_COLORS.length] ?? "bg-gray-500";
}
const SEAT_RINGS = ["ring-sky-500", "ring-rose-500", "ring-violet-500", "ring-teal-500"] as const;
function seatRing(slot: number): string {
  return SEAT_RINGS[slot % SEAT_RINGS.length] ?? "ring-gray-500";
}
function seatName(slot: number): string {
  return names[slot] ?? t.match.player(slot + 1);
}
function isBotSeat(slot: number): boolean {
  return header.seats.find((seat) => seat.slot === slot)?.botKind !== undefined;
}
function seatPlayer(slot: number) {
  return header.seats.find((seat) => seat.slot === slot)?.player;
}

const turnLine = computed(() => {
  const turn = view.value?.turnSlot;
  if (!isGroup || !isPlaying.value || turn === undefined) return null;
  const texts = t.games.wordHunt.board;
  if (gaveUp.value) return texts.gaveUpWatching;
  if (myTurn.value) return texts.yourTurn;
  return isBotSeat(turn) ? texts.botThinking(seatName(turn)) : texts.playerTurn(seatName(turn));
});

const typed = ref("");
const submitting = ref(false);
watch(
  () => [guesses.value.length, isPlaying.value],
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
  if (guess.length < wordLength.value) return refuse(t.games.wordHunt.board.notEnoughLetters);
  const known = words.value?.get(wordLength.value);
  if (known && !known.has(guess)) return refuse(t.games.wordHunt.board.notInWordList);
  const command = {
    game: { case: "wordHunt", value: { command: { case: "guess", value: { word: guess } } } },
  } as const;
  // The rules answer at once (hard mode, a word already tried), offline and without a round trip.
  const refusal = props.controller.refusal(command);
  if (refusal !== null) return refuse(refusalText(refusal));
  submitting.value = true;
  try {
    await props.controller.submit(command);
  } catch (error) {
    refuse(errorMessage(error));
  } finally {
    submitting.value = false;
  }
}

/** From the game, so a reload or a reconnect still knows the seat dropped out. */
const gaveUp = computed(() => {
  const slot = props.controller.mySlot;
  const playing = view.value?.playingSlots;
  return isGroup && slot !== null && playing !== undefined && !playing.includes(slot);
});
const confirmingGiveUp = ref(false);
const GIVE_UP = { game: { case: "wordHunt", value: { command: { case: "giveUp", value: {} } } } } as const;
const canGiveUp = computed(() => props.controller.refusal(GIVE_UP) === null && !gaveUp.value);
/** What giving up leads to, asked of the rules: who would win, or whether the others play on. */
const giveUpMessage = computed(() => {
  const preview = props.controller.preview(GIVE_UP);
  const texts = t.games.wordHunt.board;
  if (preview.kind === "continues") return texts.giveUpOthersPlayOn;
  if (preview.kind === "refused" || !isGroup) return texts.giveUpLoses;
  const [winner] = preview.result.winnerSlots;
  return preview.result.kind === OutcomeKind.WINNER && winner !== undefined
    ? texts.giveUpOtherWins(seatName(winner))
    : texts.giveUpNoWinner;
});

async function giveUp(): Promise<void> {
  confirmingGiveUp.value = false;
  try {
    await props.controller.submit({
      game: { case: "wordHunt", value: { command: { case: "giveUp", value: {} } } },
    });
  } catch (error) {
    refuse(errorMessage(error));
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
  const texts = t.games.wordHunt;
  const kind =
    current.mode === WordHuntPuzzleMode.DAILY
      ? texts.board.dailySubtitle(formatDay(current.day))
      : texts.facts.letters(current.wordLength);
  return current.hardMode ? `${kind} · ${texts.facts.hardMode}` : kind;
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
    if (!game) return;
    const progress: StatsProgress =
      phase === WordHuntPhase.SOLVED
        ? { kind: "solved", guesses: guessCount }
        : phase === WordHuntPhase.FAILED
          ? { kind: "lost" }
          : { kind: "playing" };
    stats.value = recordProgress(game, guessCount, progress) ?? stats.value;
  },
  { immediate: true },
);
const shownStats = computed(
  () =>
    stats.value ??
    (settings.value && !isPlaying.value && !isGroup ? statsFor(settings.value.language) : null),
);
const statsLine = computed(() => {
  const current = shownStats.value;
  if (!current) return null;
  return t.games.wordHunt.board.stats({
    played: current.played,
    winPercent: current.played > 0 ? Math.round((100 * current.won) / current.played) : 0,
    streak: current.streak,
    bestStreak: current.bestStreak,
  });
});

/** A group's scroller shows six rows whatever the solo "Guesses" setting says. */
const gridSize = computed(() => ({
  "--letters": wordLength.value,
  "--rows": isGroup ? 6 : maxGuesses.value,
}));
</script>

<template>
  <div v-if="view && settings" class="wh-layout flex min-h-0 flex-1 flex-col items-center gap-3">
    <div class="flex flex-col items-center gap-3">
      <p class="text-sm text-gray-400">{{ subtitle }}</p>
      <p v-if="turnLine" class="text-sm font-semibold" :class="myTurn ? 'text-emerald-300' : 'text-gray-300'">
        {{ turnLine }}
      </p>
      <div class="wh-grid relative flex flex-col gap-1.5" :class="{ 'wh-group': isGroup }" :style="gridSize">
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
            <!-- Who guessed: the player's avatar ringed in their seat colour, a robot for bots. -->
            <span
              v-if="isGroup"
              class="flex size-7 shrink-0 items-center justify-center rounded-full ring-2"
              :class="row.by === null ? 'invisible ring-transparent' : seatRing(row.by)"
              :title="row.by === null ? undefined : seatName(row.by)"
            >
              <template v-if="row.by !== null">
                <span
                  v-if="isBotSeat(row.by)"
                  class="flex size-6 items-center justify-center rounded-full text-white"
                  :class="seatColor(row.by)"
                >
                  <BotIcon class="size-4" />
                </span>
                <Avatar
                  v-else
                  :url="seatPlayer(row.by)?.avatarUrl ?? ''"
                  :name="seatName(row.by)"
                  :player-id="seatPlayer(row.by)?.id"
                />
              </template>
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
          <p class="text-xs font-semibold uppercase tracking-widest text-gray-400">
            {{ t.games.wordHunt.board.theWordWas }}
          </p>
          <div class="flex gap-1.5">
            <div
              v-for="(letter, index) in missedAnswer"
              :key="index"
              class="flex size-(--tile) items-center justify-center rounded-lg text-[calc(var(--tile)*0.5)] font-bold uppercase shadow-lg shadow-black/40"
              :class="[inPlaceClass(), prefersReducedMotion() ? '' : 'wh-reveal']"
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
      <p v-if="statsLine && !isPlaying" class="text-sm text-gray-400">{{ statsLine }}</p>
    </div>
    <button
      v-if="canGiveUp"
      type="button"
      class="rounded-lg border border-white/15 px-3 py-1.5 text-sm text-gray-300 hover:bg-white/5"
      @click="confirmingGiveUp = true"
    >
      {{ t.games.wordHunt.board.giveUp }}
    </button>
    <ConfirmDialog
      v-if="confirmingGiveUp"
      :title="t.games.wordHunt.board.giveUpQuestion"
      :message="giveUpMessage"
      :confirm-label="t.games.wordHunt.board.giveUp"
      :cancel-label="t.games.wordHunt.board.keepGuessing"
      @confirm="giveUp"
      @cancel="confirmingGiveUp = false"
    />
    <WordHuntKeyboard
      v-if="controller.mySlot !== null"
      class="wh-keyboard mt-4 pointer-coarse:mt-auto"
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
/* In a group the author marker takes about two rem of every row's width. */
.wh-grid.wh-group {
  --tile: max(
    1.6rem,
    min(
      6.5rem,
      calc((100vw - 4.5rem) / var(--letters) - 0.375rem),
      calc((100dvh - 20rem) / var(--rows) - 0.375rem)
    )
  );
}
/* A group's guesses scroll within about six rows, so the keyboard stays in place. */
.wh-scroll {
  max-height: calc(6 * (var(--tile) + 0.375rem));
}
.wh-grid {
  --tile: max(
    1.75rem,
    min(
      6.5rem,
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
