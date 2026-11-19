<script setup lang="ts">
import { t } from "@/i18n";
</script>

<template>
  <h3>Goal</h3>
  <p>Find and sink the other side's whole fleet before yours goes down.</p>
  <h3>Your fleet</h3>
  <ul>
    <li>Ten ships: one of four cells, two of three, three of two and four of one.</li>
    <li>Ships are straight and may not touch each other, not even at a corner.</li>
    <li>
      Tap cells to place decks (lit cells are the ones allowed), tap a deck to take it away, or press
      <b>{{ t.games.seaBattle.fleetEditor.random }}</b>. Then press
      <b>{{ t.games.seaBattle.fleetEditor.ready }}</b>.
    </li>
  </ul>
  <h3>Battle</h3>
  <ul>
    <li>Take turns firing at the other sea. A hit lets you fire again; a miss passes the turn.</li>
    <li>A sunk ship is shown whole, and the water around it is marked: no ship can be there.</li>
    <li>The marks above each sea show the ships still afloat.</li>
  </ul>
</template>
