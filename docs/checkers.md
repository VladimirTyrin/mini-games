# Checkers

Russian draughts (шашки) for two seats, in `crates/games/checkers`. The rules are fixed; the only
setting is who plays white (the host, the guest, or drawn by lot when the match starts).

## Board

8×8, played on the dark squares. Squares are numbered `y * 8 + x`: `0` is a1, white's left
corner, `y` grows towards black. Each side starts with twelve men on its three nearest rows;
white moves first. A player who plays black sees the board turned round.

## Moves

- A man moves one square diagonally forward and captures diagonally in all four directions,
  backwards too.
- A king (a man that reaches the far row) is flying: it moves and captures along a whole
  diagonal, jumping one enemy piece and landing on any empty square behind it. When some of those
  landings let it capture again, it must land on one of them.
- Capturing is compulsory, but any capturing sequence may be chosen, not necessarily the longest.
  A capture goes on while it can.
- Captured pieces stay on the board until the move is over: they cannot be jumped twice and they
  block the way (the Turkish strike).
- A man that reaches the far row in the middle of a capture is crowned there and goes on capturing
  as a king.

A move is sent as its whole path: the start square and every landing (at most 13 squares). The
game checks it against the generated legal moves; nothing else is accepted.

## End

- **Win**: the opponent has no pieces, or cannot move, or left the match.
- **Draw**: the same position (pieces and side to move) comes up a third time, or fifteen moves
  each (30 in a row) go by with only kings moving and nothing captured. A capture or a man's move
  starts both counts over, since no earlier position can come back after it.

## Bots

- **Easy**: a random legal move, half the time the one taking the most pieces.
- **Medium**: alpha-beta three plies deep.
- **Hard**: iterative-deepening alpha-beta within its node budget (250 000 positions, about a
  quarter of a second natively at worst).

Both search bots extend captures, so a search never stops in the middle of an exchange, and pick
at random among equally good moves. The evaluation counts material (a king is worth three men),
advancement, the home row, the centre and the long diagonal.

## Client

The board (`client/src/games/checkers/CheckersBoard.vue`) picks a move square by square from the
legal moves in the view: tap a piece, then each landing; a tap on a later landing goes straight
there when only one route reaches it (`path-choice.ts`). When only one piece can move it is picked
already, so a forced capture is a single tap.
