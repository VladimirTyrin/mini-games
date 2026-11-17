/// The single list of games. Every per-game dispatch in this crate is generated from it.
///
/// Entry: `Kind => game type, GameSettings oneof variant, bots | no_bots;`
/// `cargo xtask new-game` inserts new entries above the marker line.
macro_rules! games {
    ($callback:ident) => {
        $callback! {
            Snake => mg_snake::Snake, Snake, bots;
            TicTacToe => mg_tictactoe::TicTacToe, Tictactoe, bots;
            StackAttack => mg_stack_attack::StackAttack, StackAttack, no_bots;
            Puzzle2048 => mg_puzzle2048::Puzzle2048, Puzzle2048, no_bots;
            NumbersMatch => mg_numbers_match::NumbersMatch, NumbersMatch, no_bots;
            WordHunt => mg_word_hunt::WordHunt, WordHunt, bots;
            Checkers => mg_checkers::Checkers, Checkers, bots;
            SeaBattle => mg_sea_battle::SeaBattle, SeaBattle, bots;
            // xtask:games
        }
    };
}

pub(crate) use games;
