import { CheckersRefusal } from "@/proto/minigames/v2/checkers_pb";
import { NumbersMatchRefusal } from "@/proto/minigames/v2/numbers_match_pb";
import { Puzzle2048Refusal } from "@/proto/minigames/v2/puzzle2048_pb";
import { CommonRefusal, SettingRule } from "@/proto/minigames/v2/refusal_pb";
import { SeaBattleRefusal } from "@/proto/minigames/v2/sea_battle_pb";
import { SnakeRefusal } from "@/proto/minigames/v2/snake_pb";
import { TicTacToeRefusal } from "@/proto/minigames/v2/tictactoe_pb";
import { WordHuntRefusal } from "@/proto/minigames/v2/word_hunt_pb";
// xtask:imports

/**
 * Why the rules refuse a move or settings, by the core's enums (`refusal.proto`). Records of the
 * enums: a new reason without words in every language does not compile.
 */
export const refusals = {
  /** A reason this app does not know yet (a newer server). */
  notAllowed: "That is not allowed",
  common: {
    [CommonRefusal.UNSPECIFIED]: "That is not allowed",
    [CommonRefusal.GAME_OVER]: "The game is over",
    [CommonRefusal.NOT_YOUR_TURN]: "Not your turn",
    [CommonRefusal.NOT_PLAYING]: "You are not playing",
    [CommonRefusal.BUSY]: "Too many moves at once",
    [CommonRefusal.MALFORMED]: "That is not allowed",
    [CommonRefusal.PAUSED]: "The match is paused",
  } satisfies Record<CommonRefusal, string>,
  snake: {
    [SnakeRefusal.UNSPECIFIED]: "That is not allowed",
    [SnakeRefusal.ALREADY_HEADING_THAT_WAY]: "Already heading that way",
    [SnakeRefusal.CANNOT_REVERSE]: "A snake cannot turn back",
  } satisfies Record<SnakeRefusal, string>,
  tictactoe: {
    [TicTacToeRefusal.UNSPECIFIED]: "That is not allowed",
    [TicTacToeRefusal.CELL_TAKEN]: "That cell is taken",
  } satisfies Record<TicTacToeRefusal, string>,
  puzzle2048: {
    [Puzzle2048Refusal.UNSPECIFIED]: "That is not allowed",
    [Puzzle2048Refusal.MOVE_CHANGES_NOTHING]: "That move changes nothing",
  } satisfies Record<Puzzle2048Refusal, string>,
  numbersMatch: {
    [NumbersMatchRefusal.UNSPECIFIED]: "That is not allowed",
    [NumbersMatchRefusal.NOT_DEALT_YET]: "The board is not dealt yet",
    [NumbersMatchRefusal.SAME_CELL]: "Pick two different cells",
    [NumbersMatchRefusal.CELL_CROSSED_OUT]: "That cell is already crossed out",
    [NumbersMatchRefusal.NOT_A_PAIR]: "Not a pair: equal numbers or a sum of 10",
    [NumbersMatchRefusal.TOO_FAR_APART]:
      "Too far apart: pairs must touch, or have only crossed-out cells between them",
    [NumbersMatchRefusal.NO_REFILLS_LEFT]: "No refills left",
    [NumbersMatchRefusal.HINTS_DISABLED]: "Hints are off",
    [NumbersMatchRefusal.NO_HINTS_LEFT]: "No hints left",
    [NumbersMatchRefusal.HINT_ALREADY_SHOWN]: "A hint is already shown",
  } satisfies Record<NumbersMatchRefusal, string>,
  wordHunt: {
    [WordHuntRefusal.UNSPECIFIED]: "That is not allowed",
    [WordHuntRefusal.WORD_NOT_CHOSEN_YET]: "The word is not chosen yet",
    [WordHuntRefusal.LAST_GUESS_NOT_MARKED]: "The last guess is not marked yet",
    [WordHuntRefusal.WRONG_LENGTH]: "The word has the wrong number of letters",
    [WordHuntRefusal.NOT_A_LETTER]: "The word has a character outside the alphabet",
    [WordHuntRefusal.ALREADY_TRIED]: "This word was already tried",
    [WordHuntRefusal.MOVED_CORRECT_LETTER]: "Hard mode: keep every green letter in its place",
    [WordHuntRefusal.DROPPED_FOUND_LETTER]: "Hard mode: use every letter you have found",
    [WordHuntRefusal.CANNOT_GIVE_UP]: "Only a player still guessing can give up",
  } satisfies Record<WordHuntRefusal, string>,
  checkers: {
    [CheckersRefusal.UNSPECIFIED]: "That is not allowed",
    [CheckersRefusal.MUST_CAPTURE]: "You must capture",
    [CheckersRefusal.NOT_A_LEGAL_MOVE]: "Not a legal move",
  } satisfies Record<CheckersRefusal, string>,
  seaBattle: {
    [SeaBattleRefusal.UNSPECIFIED]: "That is not allowed",
    [SeaBattleRefusal.BATTLE_NOT_STARTED]: "The battle has not started yet",
    [SeaBattleRefusal.LAST_SHOT_NOT_RESOLVED]: "The last shot is not resolved yet",
    [SeaBattleRefusal.CELL_ALREADY_KNOWN]: "That cell is known already",
    [SeaBattleRefusal.FLEET_ALREADY_PLACED]: "The fleet is placed already",
    [SeaBattleRefusal.WRONG_SHIPS]: "A fleet is one ship of 4, two of 3, three of 2 and four of 1",
    [SeaBattleRefusal.SHIPS_TOUCH]: "Ships may not touch, not even at a corner",
  } satisfies Record<SeaBattleRefusal, string>,
  // xtask:refusals
  settingRules: {
    [SettingRule.UNSPECIFIED]: "This value cannot be used",
    [SettingRule.POWER_OF_TWO]: "Must be a power of two",
    [SettingRule.TOO_LARGE_FOR_BOARD]: "Cannot be reached on a board this small",
    [SettingRule.FIXED_FOR_DAILY_PUZZLE]: "Fixed for the word of the day",
  } satisfies Record<SettingRule, string>,
  outOfRange: (min: number, max: number) => `Must be from ${min} to ${max}`,
  /** Settings that do not suit the players seated (a single-player mode with a guest, say). */
  doNotSuitPlayers: "These settings do not suit this many players",
};
