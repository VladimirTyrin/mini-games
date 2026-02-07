use crate::state::PlayAgainStatus;
use crate::CommandSender;
use common::{GameStateUpdate, ScoreEntry, PlayerIdentity};
use eframe::egui;
use std::path::PathBuf;

pub enum GameUi {
    Snake(super::snake::SnakeGameUi),
    TicTacToe(super::tictactoe::TicTacToeGameUi),
    NumbersMatch(super::numbers_match::NumbersMatchGameUi),
    Puzzle2048(super::puzzle2048::Puzzle2048GameUi),
}

impl GameUi {
    pub fn new_snake() -> Self {
        GameUi::Snake(super::snake::SnakeGameUi::new())
    }

    pub fn new_tictactoe() -> Self {
        GameUi::TicTacToe(super::tictactoe::TicTacToeGameUi::new())
    }

    pub fn new_numbers_match() -> Self {
        GameUi::NumbersMatch(super::numbers_match::NumbersMatchGameUi::new())
    }

    pub fn new_puzzle2048() -> Self {
        GameUi::Puzzle2048(super::puzzle2048::Puzzle2048GameUi::new())
    }

    pub fn render_game(
        &mut self,
        egui_ui: &mut egui::Ui,
        ctx: &egui::Context,
        session_id: &str,
        game_state: &Option<GameStateUpdate>,
        client_id: &str,
        is_observer: bool,
        command_sender: &CommandSender,
        force_show_dead: bool,
        highlighted_pair: Option<(u32, u32)>,
    ) {
        match self {
            GameUi::Snake(ui) => ui.render_game(egui_ui, ctx, session_id, game_state, client_id, is_observer, command_sender, force_show_dead),
            GameUi::TicTacToe(ui) => ui.render_game(egui_ui, ctx, session_id, game_state, client_id, is_observer, command_sender),
            GameUi::NumbersMatch(ui) => ui.render_game(egui_ui, ctx, session_id, game_state, client_id, is_observer, command_sender, highlighted_pair),
            GameUi::Puzzle2048(ui) => ui.render_game(egui_ui, ctx, session_id, game_state, client_id, is_observer, command_sender),
        }
    }

    pub fn render_game_over_snake(
        &mut self,
        egui_ui: &mut egui::Ui,
        ctx: &egui::Context,
        scores: &[ScoreEntry],
        winner: &Option<PlayerIdentity>,
        client_id: &str,
        last_game_state: &Option<GameStateUpdate>,
        game_info: &common::proto::snake::SnakeGameEndInfo,
        play_again_status: &PlayAgainStatus,
        is_observer: bool,
        command_sender: &CommandSender,
        replay_path: Option<&PathBuf>,
    ) -> bool {
        match self {
            GameUi::Snake(ui) => ui.render_game_over(
                egui_ui,
                ctx,
                scores,
                winner,
                client_id,
                last_game_state,
                game_info,
                play_again_status,
                is_observer,
                command_sender,
                replay_path,
            ),
            GameUi::TicTacToe(_) | GameUi::NumbersMatch(_) | GameUi::Puzzle2048(_) => false,
        }
    }

    pub fn render_game_over_tictactoe(
        &mut self,
        egui_ui: &mut egui::Ui,
        ctx: &egui::Context,
        scores: &[ScoreEntry],
        winner: &Option<PlayerIdentity>,
        client_id: &str,
        last_game_state: &Option<GameStateUpdate>,
        game_info: &common::proto::tictactoe::TicTacToeGameEndInfo,
        play_again_status: &PlayAgainStatus,
        is_observer: bool,
        command_sender: &CommandSender,
        replay_path: Option<&PathBuf>,
    ) -> bool {
        match self {
            GameUi::Snake(_) | GameUi::NumbersMatch(_) | GameUi::Puzzle2048(_) => false,
            GameUi::TicTacToe(ui) => ui.render_game_over(
                egui_ui,
                ctx,
                scores,
                winner,
                client_id,
                last_game_state,
                game_info,
                play_again_status,
                is_observer,
                command_sender,
                replay_path,
            ),
        }
    }

    pub fn render_game_over_numbers_match(
        &mut self,
        egui_ui: &mut egui::Ui,
        ctx: &egui::Context,
        scores: &[ScoreEntry],
        winner: &Option<PlayerIdentity>,
        client_id: &str,
        last_game_state: &Option<GameStateUpdate>,
        game_info: &common::proto::numbers_match::NumbersMatchGameEndInfo,
        play_again_status: &PlayAgainStatus,
        is_observer: bool,
        command_sender: &CommandSender,
        replay_path: Option<&PathBuf>,
    ) -> bool {
        match self {
            GameUi::Snake(_) | GameUi::TicTacToe(_) | GameUi::Puzzle2048(_) => false,
            GameUi::NumbersMatch(ui) => ui.render_game_over(
                egui_ui,
                ctx,
                scores,
                winner,
                client_id,
                last_game_state,
                game_info,
                play_again_status,
                is_observer,
                command_sender,
                replay_path,
            ),
        }
    }

    pub fn render_game_over_puzzle2048(
        &mut self,
        egui_ui: &mut egui::Ui,
        ctx: &egui::Context,
        scores: &[ScoreEntry],
        winner: &Option<PlayerIdentity>,
        client_id: &str,
        last_game_state: &Option<GameStateUpdate>,
        game_info: &common::proto::puzzle2048::Puzzle2048GameEndInfo,
        play_again_status: &PlayAgainStatus,
        is_observer: bool,
        command_sender: &CommandSender,
        replay_path: Option<&PathBuf>,
    ) -> bool {
        match self {
            GameUi::Snake(_) | GameUi::TicTacToe(_) | GameUi::NumbersMatch(_) => false,
            GameUi::Puzzle2048(ui) => ui.render_game_over(
                egui_ui,
                ctx,
                scores,
                winner,
                client_id,
                last_game_state,
                game_info,
                play_again_status,
                is_observer,
                command_sender,
                replay_path,
            ),
        }
    }
}
