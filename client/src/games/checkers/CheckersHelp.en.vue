<template>
  <h3>Goal</h3>
  <p>Take all the other side's pieces or leave them without a move. White moves first.</p>
  <h3>Rules (Russian draughts)</h3>
  <ul>
    <li>Pieces move one square diagonally forward, on the dark squares only.</li>
    <li>
      Capturing is <b>compulsory</b>: jump over an enemy piece to the empty square behind it, backwards too.
      Keep jumping while you can; you may choose which way.
    </li>
    <li>
      A piece that reaches the far row becomes a <b>king</b>. Kings fly: they move and capture along a whole
      diagonal and may land on any free square beyond the piece taken.
    </li>
    <li>
      A piece that reaches the far row in the middle of a capture becomes a king at once and goes on capturing
      as one.
    </li>
    <li>Captured pieces leave the board only when the move is over, so a piece can't be jumped twice.</li>
    <li>
      The game is a draw when the same position comes up three times, or after 15 moves each with only kings
      moving and nothing taken.
    </li>
  </ul>
</template>
