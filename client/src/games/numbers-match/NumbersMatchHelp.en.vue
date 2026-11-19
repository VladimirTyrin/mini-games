<script setup lang="ts">
import { t } from "@/i18n";

import { PAIR_EXAMPLES } from "./pair-examples";
import PairExample from "./PairExample.vue";
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
    <PairExample v-bind="PAIR_EXAMPLES.row" caption="Neighbours in a row: 3 + 7 = 10" />
    <PairExample v-bind="PAIR_EXAMPLES.column" caption="Neighbours in a column: 6 and 6 are equal" />
    <PairExample v-bind="PAIR_EXAMPLES.diagonal" caption="Neighbours on a diagonal: 2 + 8 = 10" />
    <PairExample
      v-bind="PAIR_EXAMPLES.acrossCrossedOut"
      caption="Across crossed-out cells: they no longer stand between"
    />
    <PairExample
      v-bind="PAIR_EXAMPLES.rowEndToNext"
      caption="From the end of a row to the start of the next"
    />
    <PairExample
      v-bind="PAIR_EXAMPLES.acrossCrossedOutAndRowEnd"
      caption="Across crossed-out cells and the end of a row: 8 + 2 = 10"
    />
    <PairExample v-bind="PAIR_EXAMPLES.notMatching" caption="Not a pair: 3 and 5 differ and make 8, not 10" />
    <PairExample
      v-bind="PAIR_EXAMPLES.notNeighbours"
      caption="Not neighbours: the 6 between them is still in play"
    />
  </div>
  <h3>Stuck?</h3>
  <ul>
    <li>
      <b>{{ t.games.numbersMatch.board.refill }}</b> copies every number still in play to the end of the
      board, in a new colour. Refills are limited.
    </li>
    <li>
      <b>{{ t.games.numbersMatch.board.hint }}</b> lights up a pair you can take, when the game's settings
      allow hints.
    </li>
    <li>You lose when no pair is left and there are no refills.</li>
  </ul>
</template>
