import type { Messages } from "../../messages";

export const tictactoe: Messages["games"]["tictactoe"] = {
  title: "Крестики-нолики",
  tagline: "N в ряд на большом поле",
  inARow: (count) => `${count} в ряд`,
  facts: {
    hostStarts: "начинает хост",
    guestStarts: "начинает гость",
    randomStart: "первый ход по жребию",
  },
  bots: {
    minimax: "Умный бот",
    random: "Случайный бот",
  },
  controls: {
    keyboard: "Щёлкни по клетке, чтобы поставить свой знак",
    touch: "Нажми на клетку, чтобы поставить свой знак",
  },
  settings: {
    inARowToWin: "В ряд для победы",
    firstMove: "Первый ход",
    firstMoveHelp: "Играет крестиками",
  },
  board: {
    draw: "Ничья",
    nobodyWins: "Победителя нет",
    wins: (name) => `Победа: ${name}`,
    opponentLeft: "Соперник вне игры — победа",
    gameOver: "Игра окончена",
    youWin: "Победа",
    youLose: "Поражение",
    choosingFirst: "Выбираем, кто начнёт…",
    playersTurn: (name) => `Ходит ${name}`,
    yourTurn: "Твой ход",
    opponentsTurn: "Ход соперника",
    youPlayBeforeMark: "(ты играешь",
    youPlayAfterMark: ")",
  },
  marks: {
    x: "Крестик",
    o: "Нолик",
  },
};
