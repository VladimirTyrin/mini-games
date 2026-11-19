import type { Messages } from "../../messages";

const DECKS: Readonly<Record<number, string>> = {
  1: "Однопалубный",
  2: "Двухпалубный",
  3: "Трёхпалубный",
  4: "Четырёхпалубный",
};

export const seaBattle: Messages["games"]["seaBattle"] = {
  title: "Морской бой",
  tagline: "Найди и потопи скрытый флот",
  facts: { classicRules: "классические правила" },
  bots: { easy: "Лёгкий бот", medium: "Средний бот", hard: "Сильный бот" },
  controls: {
    keyboard: "Щёлкай по клеткам, чтобы расставить флот, затем стреляй по морю соперника",
    touch: "Нажимай на клетки, чтобы расставить флот, затем стреляй по морю соперника",
  },
  endReasons: {
    fleetSunk: "Весь флот потоплен",
    gaveUpOrLeft: "Игрок сдался или вышел из игры",
  },
  columns: "АБВГДЕЖЗИК",
  ship: (length) => `${DECKS[length] ?? `${length}-палубный`} корабль`,
  board: {
    seat: (number) => `Место ${number}`,
    yourFleet: "Твой флот",
    opponentsSea: (name) => `Море · ${name}`,
    opponent: "Соперник",
    shipsAfloat: "Корабли на плаву",
    fleetsBeingPlaced: "Идёт расстановка флотов…",
    waitingForOpponentsFleet: "Ждём флот соперника…",
    placeYourFleet: "Расставь флот",
    placingRandomFleet: "Расставляем флот случайно…",
    placeYourFleetWithin: (time) => `Расставь флот · ${time}`,
    playerFires: (name) => `Стреляет ${name}`,
    yourShot: "Твой выстрел",
    opponentsShot: "Стреляет соперник",
    draw: "Ничья",
    playerWins: (name) => `Победа: ${name}`,
    opponentLeftYouWin: "Соперник вне игры — ты выиграл",
    youWin: "Ты выиграл",
    youLose: "Ты проиграл",
  },
  fleetEditor: {
    shipsToPlace: "Корабли для расстановки",
    noRoom: "Остальным кораблям не хватает места: убери несколько палуб",
    hint: "Нажми на светлую клетку, чтобы добавить палубу, на корабль — чтобы убрать",
    random: "Случайно",
    clear: "Очистить",
    ready: "Готово",
  },
  settings: {
    classicRules:
      "Классические правила: поле 10×10 и десять кораблей — один четырёхпалубный, два трёхпалубных, три двухпалубных, четыре однопалубных.",
    noTouching: "Корабли не касаются друг друга, даже углами. Попал — стреляешь ещё раз.",
  },
};
