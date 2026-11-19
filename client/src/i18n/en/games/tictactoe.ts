export const tictactoe = {
  title: "Tic-Tac-Toe",
  tagline: "N in a row on a big board",
  inARow: (count: number) => `${count} in a row`,
  facts: {
    hostStarts: "host starts",
    guestStarts: "guest starts",
    randomStart: "random start",
  },
  bots: {
    minimax: "Minimax bot",
    random: "Random bot",
  },
  controls: {
    keyboard: "Click a cell to place your mark",
    touch: "Tap a cell to place your mark",
  },
  settings: {
    inARowToWin: "In a row to win",
    firstMove: "First move",
    firstMoveHelp: "Plays X",
  },
  board: {
    draw: "Draw",
    nobodyWins: "Nobody wins",
    wins: (name: string) => `${name} wins`,
    opponentLeft: "Opponent left — you win",
    gameOver: "Game over",
    youWin: "You win!",
    youLose: "You lose",
    choosingFirst: "Choosing who starts…",
    playersTurn: (name: string) => `${name}'s turn`,
    yourTurn: "Your turn",
    opponentsTurn: "Opponent's turn",
    // The player's mark is drawn between the two parts.
    youPlayBeforeMark: "(you play",
    youPlayAfterMark: ")",
  },
  marks: {
    x: "X",
    o: "O",
  },
};
