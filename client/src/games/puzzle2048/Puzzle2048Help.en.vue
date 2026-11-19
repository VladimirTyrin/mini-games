<template>
  <h3>Goal</h3>
  <p>Merge tiles until one reaches the target number (2048 by default).</p>
  <h3>Rules</h3>
  <ul>
    <li>Every move slides all the tiles as far as they go in one direction.</li>
    <li>Two equal tiles that meet merge into one with their sum: 2 and 2 make 4, 4 and 4 make 8.</li>
    <li>After each move a new 2 (sometimes a 4) appears on a free cell.</li>
    <li>The game is over when the board is full and no move merges anything.</li>
  </ul>
  <h3>Tip</h3>
  <p>Keep your biggest tile in a corner and build towards it along one edge.</p>
</template>
