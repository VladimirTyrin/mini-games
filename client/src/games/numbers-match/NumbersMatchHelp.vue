<script setup lang="ts">
import PairExample from "./PairExample.vue";

// Rows of 9 like the real board: a number, 0 for a crossed-out cell, null for an empty one.
const EXAMPLES = [
  {
    caption: "Neighbours in a row: 3 + 7 = 10",
    cells: [5, 3, 7, 1, 9, 4, 2, 6, 8],
    pair: [1, 2],
    allowed: true,
  },
  {
    caption: "Neighbours in a column: 6 and 6 are equal",
    cells: [2, 6, 1, 9, 3, 7, 5, 8, 4, 4, 6, 8, 2, 5, 1, 9, 3, 7],
    pair: [1, 10],
    allowed: true,
  },
  {
    caption: "Neighbours on a diagonal: 2 + 8 = 10",
    cells: [2, 5, 9, 1, 4, 6, 3, 7, 5, 1, 8, 3, 6, 9, 2, 4, 5, 7],
    pair: [0, 10],
    allowed: true,
  },
  {
    caption: "Across crossed-out cells: they no longer stand between",
    cells: [7, 4, 0, 0, 0, 4, 9, 2, 1],
    pair: [1, 5],
    allowed: true,
  },
  {
    caption: "From the end of a row to the start of the next",
    cells: [1, 5, 9, 2, 7, 3, 8, 2, 6, 4, 1, 3, 7, 9, 2, 5, 8, 1],
    pair: [8, 9],
    allowed: true,
  },
  {
    caption: "Across crossed-out cells and the end of a row: 8 + 2 = 10",
    cells: [2, 7, 4, 1, 9, 3, 8, 0, 0, 0, 0, 2, 6, 5, 1, 7, 4, 9],
    pair: [6, 11],
    allowed: true,
  },
  {
    caption: "Not a pair: 3 and 5 differ and make 8, not 10",
    cells: [9, 3, 5, 1, 7, 2, 4, 6, 8],
    pair: [1, 2],
    allowed: false,
  },
  {
    caption: "Not neighbours: the 6 between them is still in play",
    cells: [2, 1, 6, 9, 4, 3, 7, 5, 8],
    pair: [1, 3],
    allowed: false,
  },
] as const;
</script>

<template>
  <h3>Goal</h3>
  <p>Clear every number off the board.</p>
  <h3>Pairs</h3>
  <ul>
    <li>
      Pick two numbers that are <b>equal</b> (4 and 4) or <b>add up to 10</b> (3 and 7) to cross them out.
    </li>
    <li>
      They must be <b>neighbours</b>: side by side, one above the other or on a diagonal, or the last number
      of a row and the first of the next. Crossed-out cells between them don't count, so a pair can reach
      across them, in a line or on from the end of a row to the next one.
    </li>
    <li>A row with every number crossed out disappears and the rows below move up.</li>
  </ul>
  <h3>What you can do</h3>
  <div class="mt-2 flex flex-col gap-4">
    <PairExample
      v-for="example in EXAMPLES"
      :key="example.caption"
      :cells="example.cells"
      :pair="example.pair"
      :allowed="example.allowed"
      :caption="example.caption"
    />
  </div>
  <h3>Stuck?</h3>
  <ul>
    <li>
      <b>Refill</b> copies every number still in play to the end of the board, in a new colour. Refills are
      limited.
    </li>
    <li><b>Hint</b> lights up a pair you can take, when the game's settings allow hints.</li>
    <li>You lose when no pair is left and there are no refills.</li>
  </ul>
</template>
