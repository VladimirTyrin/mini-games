import type { Messages } from "../../messages";

export const puzzle2048: Messages["games"]["puzzle2048"] = {
  title: "2048",
  tagline: "Сдвигай и соединяй плитки, чтобы дойти до цели",
  toTarget: (target) => `до ${target}`,
  controls: {
    keyboard: "Стрелки или WASD — сдвинуть плитки",
    touch: "Проведи пальцем, чтобы сдвинуть плитки",
  },
  settings: {
    targetTile: "Цель",
    targetTileHelp: "Собери такую плитку, чтобы победить",
    customTarget: (target) => `Своя (${target})`,
  },
  board: {
    score: "Очки",
    moves: "Ходы",
    target: "Цель",
    reachedTarget: (target) => `Плитка ${target} собрана`,
    noMovesLeft: "Ходов нет",
    abandoned: "Игра прервана",
  },
};
