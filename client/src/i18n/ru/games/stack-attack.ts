import type { Messages } from "../../messages";
import { pluralRu } from "../../plural";

export const stackAttack: Messages["games"]["stackAttack"] = {
  title: "Stack Attack",
  tagline: "Кооператив: выживи под падающими ящиками",
  facts: {
    coop: "кооператив",
  },
  controls: {
    keyboard:
      "Держи ←/→ или A/D, чтобы идти и толкать ящики · Пробел, W или ↑ — прыжок · прыгни и иди, чтобы залезть на ящик",
    touch: "Держи ◀ ▶, чтобы идти и толкать ящики · Прыгни и иди, чтобы залезть на ящик",
  },
  endReasons: {
    workersCrushed: "Раздавило падающим ящиком",
    ceilingReached: "Ящики дошли до потолка",
    allWorkersLeft: "Все грузчики ушли",
    workersTrapped: "Заперты в яме без выхода",
  },
  settings: {
    boardSizeHelp: "Больше места, чтобы ходить и толкать ящики",
  },
  board: {
    watching: "Ты зритель",
    crushedWatching: "Тебя раздавило — ты зритель",
    walkLeft: "Идти влево",
    walkRight: "Идти вправо",
    jump: "Прыгнуть",
  },
  info: {
    score: "Очки",
    level: "Уровень",
    lines: "Ряды",
    pushed: "Сдвинуто",
    time: "Время",
    you: "(ты)",
    crushed: "под ящиком",
    left: "вне игры",
    howToScore: (pointsPerLine) =>
      `Толкай ящики, чтобы заполнить ряд: полный ряд исчезает и даёт ${pointsPerLine} ${pluralRu(pointsPerLine, "очко", "очка", "очков")}. Не дай себя раздавить или замуровать в яме и держи стопку ниже красной линии.`,
    smash: "Прыгни в падающий ящик, чтобы разбить его до приземления.",
  },
};
