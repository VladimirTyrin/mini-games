import { CheckersRefusal } from "@/proto/minigames/v2/checkers_pb";
import { NumbersMatchRefusal } from "@/proto/minigames/v2/numbers_match_pb";
import { Puzzle2048Refusal } from "@/proto/minigames/v2/puzzle2048_pb";
import { CommonRefusal, SettingRule } from "@/proto/minigames/v2/refusal_pb";
import { SeaBattleRefusal } from "@/proto/minigames/v2/sea_battle_pb";
import { SnakeRefusal } from "@/proto/minigames/v2/snake_pb";
import { TicTacToeRefusal } from "@/proto/minigames/v2/tictactoe_pb";
import { WordHuntRefusal } from "@/proto/minigames/v2/word_hunt_pb";
// xtask:imports

import type { Messages } from "../messages";

export const refusals: Messages["refusals"] = {
  notAllowed: "Так нельзя",
  common: {
    [CommonRefusal.UNSPECIFIED]: "Так нельзя",
    [CommonRefusal.GAME_OVER]: "Игра окончена",
    [CommonRefusal.NOT_YOUR_TURN]: "Сейчас не твой ход",
    [CommonRefusal.NOT_PLAYING]: "Ты не играешь",
    [CommonRefusal.BUSY]: "Слишком много ходов сразу",
    [CommonRefusal.MALFORMED]: "Так нельзя",
    [CommonRefusal.PAUSED]: "Матч на паузе",
  },
  snake: {
    [SnakeRefusal.UNSPECIFIED]: "Так нельзя",
    [SnakeRefusal.ALREADY_HEADING_THAT_WAY]: "Змейка уже ползёт туда",
    [SnakeRefusal.CANNOT_REVERSE]: "Змейка не может повернуть назад",
  },
  tictactoe: {
    [TicTacToeRefusal.UNSPECIFIED]: "Так нельзя",
    [TicTacToeRefusal.CELL_TAKEN]: "Клетка уже занята",
  },
  puzzle2048: {
    [Puzzle2048Refusal.UNSPECIFIED]: "Так нельзя",
    [Puzzle2048Refusal.MOVE_CHANGES_NOTHING]: "Этот ход ничего не меняет",
  },
  numbersMatch: {
    [NumbersMatchRefusal.UNSPECIFIED]: "Так нельзя",
    [NumbersMatchRefusal.NOT_DEALT_YET]: "Поле ещё не разложено",
    [NumbersMatchRefusal.SAME_CELL]: "Выбери две разные клетки",
    [NumbersMatchRefusal.CELL_CROSSED_OUT]: "Эта клетка уже зачёркнута",
    [NumbersMatchRefusal.NOT_A_PAIR]: "Не пара: нужны равные числа или сумма 10",
    [NumbersMatchRefusal.TOO_FAR_APART]:
      "Слишком далеко: числа должны стоять рядом или разделяться только зачёркнутыми клетками",
    [NumbersMatchRefusal.NO_REFILLS_LEFT]: "Добавлений не осталось",
    [NumbersMatchRefusal.HINTS_DISABLED]: "Подсказки выключены",
    [NumbersMatchRefusal.NO_HINTS_LEFT]: "Подсказок не осталось",
    [NumbersMatchRefusal.HINT_ALREADY_SHOWN]: "Подсказка уже показана",
  },
  wordHunt: {
    [WordHuntRefusal.UNSPECIFIED]: "Так нельзя",
    [WordHuntRefusal.WORD_NOT_CHOSEN_YET]: "Слово ещё не загадано",
    [WordHuntRefusal.LAST_GUESS_NOT_MARKED]: "Прошлая попытка ещё не отмечена",
    [WordHuntRefusal.WRONG_LENGTH]: "В слове не то число букв",
    [WordHuntRefusal.NOT_A_LETTER]: "В слове есть знак не из алфавита",
    [WordHuntRefusal.ALREADY_TRIED]: "Это слово уже было",
    [WordHuntRefusal.MOVED_CORRECT_LETTER]: "Сложный режим: оставь зелёные буквы на своих местах",
    [WordHuntRefusal.DROPPED_FOUND_LETTER]: "Сложный режим: используй все найденные буквы",
    [WordHuntRefusal.CANNOT_GIVE_UP]: "Сдаться можно, только пока угадываешь",
  },
  checkers: {
    [CheckersRefusal.UNSPECIFIED]: "Так нельзя",
    [CheckersRefusal.MUST_CAPTURE]: "Бить обязательно",
    [CheckersRefusal.NOT_A_LEGAL_MOVE]: "Так ходить нельзя",
  },
  seaBattle: {
    [SeaBattleRefusal.UNSPECIFIED]: "Так нельзя",
    [SeaBattleRefusal.BATTLE_NOT_STARTED]: "Бой ещё не начался",
    [SeaBattleRefusal.LAST_SHOT_NOT_RESOLVED]: "Результат прошлого выстрела ещё неизвестен",
    [SeaBattleRefusal.CELL_ALREADY_KNOWN]: "Про эту клетку уже всё известно",
    [SeaBattleRefusal.FLEET_ALREADY_PLACED]: "Флот уже расставлен",
    [SeaBattleRefusal.WRONG_SHIPS]:
      "Флот — это один четырёхпалубный, два трёхпалубных, три двухпалубных и четыре однопалубных",
    [SeaBattleRefusal.SHIPS_TOUCH]: "Корабли не могут касаться, даже углами",
  },
  // xtask:refusals
  settingRules: {
    [SettingRule.UNSPECIFIED]: "Это значение нельзя использовать",
    [SettingRule.POWER_OF_TWO]: "Должно быть степенью двойки",
    [SettingRule.TOO_LARGE_FOR_BOARD]: "Недостижимо на таком маленьком поле",
    [SettingRule.FIXED_FOR_DAILY_PUZZLE]: "Для слова дня не меняется",
  },
  outOfRange: (min: number, max: number) => `Должно быть от ${min} до ${max}`,
  doNotSuitPlayers: "Эти настройки не подходят для такого числа игроков",
};
