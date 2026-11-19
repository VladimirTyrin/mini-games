import type { Messages } from "../../messages";
import { pluralRu } from "../../plural";

export const numbersMatch: Messages["games"]["numbersMatch"] = {
  title: "Пары чисел",
  tagline: "Убирай пары: равные числа или сумма 10",
  facts: {
    hints: (mode) => `подсказки: ${mode}`,
    refills: (count) => `${count} ${pluralRu(count, "добавление", "добавления", "добавлений")}`,
  },
  controls: {
    keyboard: "Щёлкай по двум равным числам или по двум с суммой 10 · H — подсказка · R — добавить числа",
    touch: "Нажимай на два равных числа или на два с суммой 10 · «Подсказать» и «Добавить» — под полем",
  },
  hintModes: {
    off: "Выкл",
    limited: "Лимит",
    unlimited: "Безлимит",
    unknown: "По умолчанию",
  },
  settings: {
    hints: "Подсказки",
    hintsHelp: "Показывают пару, которую можно убрать",
    refills: "Добавления",
    refillsHelp: "Сколько раз можно дописать оставшиеся числа вниз",
  },
  board: {
    cleared: "Поле очищено",
    abandoned: "Игра прервана",
    noMovesLeft: "Ходов нет",
    noPairsRefill: "Пар больше нет — добавь числа",
    numbersLeft: (count) => `Осталось ${count} ${pluralRu(count, "число", "числа", "чисел")}`,
    dealing: "Раздача…",
    pairRefused: "Эту пару убрать нельзя",
    refill: "Добавить",
    refillWithKey: "Добавить (R)",
    hint: "Подсказать",
    hintWithKey: "Подсказать (H)",
  },
};
