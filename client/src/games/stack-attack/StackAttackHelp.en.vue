<script setup lang="ts">
import { POINTS_PER_LINE } from "./points-per-line";
</script>

<template>
  <h3>Goal</h3>
  <p>
    Survive as long as you can while cranes drop crates, and clear rows for points. With friends you play
    together.
  </p>
  <h3>Rules</h3>
  <ul>
    <li>
      Walk into a crate to <b>push</b> it one cell, if the cell behind it is free. You can't push two at once.
    </li>
    <li><b>Jump</b> to get onto a crate: jump, then walk sideways onto it.</li>
    <li>A full row of crates clears for {{ POINTS_PER_LINE }} points; the crates above fall down.</li>
    <li>
      Jump into a falling crate to smash it before it lands (no points). A crate landing on your head crushes
      you.
    </li>
    <li>
      The game ends when the stack reaches the red line, when everyone is crushed, or when every worker is
      walled in.
    </li>
  </ul>
</template>
