<template>
  <h3>Goal</h3>
  <p>Guess the hidden word, Russian or English, in as few tries as you can.</p>
  <h3>Rules</h3>
  <ul>
    <li>Type a real word of the right length; it is checked as soon as its last letter is in.</li>
    <li>
      Each letter is then marked: <b class="text-emerald-400">green</b> is in the word at this place,
      <b class="text-amber-400">amber</b> is in the word but elsewhere, grey is not in the word.
    </li>
    <li>A letter repeated in your guess is marked only as many times as the word has it.</li>
    <li>
      Alone you have a fixed number of tries. In a group the players take turns on one board, and whoever
      finds the word wins.
    </li>
    <li>In <b>hard mode</b> every guess must keep the green letters in place and use the amber ones.</li>
    <li><b>Give up</b> shows the word when you play alone; in a group you leave and the others play on.</li>
  </ul>
</template>
