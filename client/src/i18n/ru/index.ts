import { common } from "./common";
import { errors } from "./errors";
import { app } from "./app";
import { home } from "./home";
import { settings } from "./settings";
import { friends } from "./friends";
import { replays } from "./replays";
import { online } from "./online";
import { lobby } from "./lobby";
import { setup } from "./setup";
import { match } from "./match";
import { snake } from "./games/snake";
import { tictactoe } from "./games/tictactoe";
import { stackAttack } from "./games/stack-attack";
import { puzzle2048 } from "./games/puzzle2048";
import { numbersMatch } from "./games/numbers-match";
import { wordHunt } from "./games/word-hunt";
import { checkers } from "./games/checkers";
import { seaBattle } from "./games/sea-battle";

import type { Messages } from "../messages";

export const ru: Messages = {
  common,
  errors,
  app,
  home,
  settings,
  friends,
  replays,
  online,
  lobby,
  setup,
  match,
  games: { snake, tictactoe, stackAttack, puzzle2048, numbersMatch, wordHunt, checkers, seaBattle },
};
