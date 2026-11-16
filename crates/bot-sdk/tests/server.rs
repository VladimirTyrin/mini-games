//! The SDK against a real `mg-server` running in-process.

use std::net::SocketAddr;
use std::time::Duration;

use mg_bot_sdk::{
    BotConfig, BotSession, BuiltinBot, LobbySetup, MatchSummary, ViewBot, ViewStrategy,
};
use mg_core::Slot;
use mg_protocol::{WireGame, proto};
use mg_server::{Config, serve};
use mg_tictactoe::{Command, FirstPlayerMode, Point, RawSettings, TicTacToe};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

/// `alpha` signs in with key `secret`, `beta` with `beta-key` (the server keeps SHA-256 only).
const BOT_KEYS: &str = "alpha:2bb80d537b1da3e38bd30361aa855686bde0eacd7162fef6a25fe97bf527a25b,\
                        beta:7a3d637bc601f7000cc2c33141c8c8a7554e02e319fa8b293a0c88c613b77620";

struct TestServer {
    address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
}

impl TestServer {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut config = <Config as clap::Parser>::parse_from(["mini_games_server"]);
        config.listen = address;
        config.bot_keys = BOT_KEYS.to_owned();
        config.timing.resume_countdown = Duration::from_millis(20);
        let (shutdown, stopped) = oneshot::channel::<()>();
        tokio::spawn(async move {
            serve(config, listener, async move { stopped.await.unwrap_or(()) })
                .await
                .unwrap();
        });
        Self {
            address,
            shutdown: Some(shutdown),
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            // The server task may already be gone at the end of a test.
            shutdown.send(()).unwrap_or(());
        }
    }
}

/// Fails a test that hangs instead of blocking the run; it never makes time pass.
async fn within<T>(scenario: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(30), scenario)
        .await
        .expect("the scenario did not finish within 30 s")
}

fn url(address: SocketAddr) -> String {
    format!("ws://{address}/ws")
}

fn small_tictactoe() -> proto::GameSettings {
    TicTacToe::settings_to_wire(&RawSettings {
        field_width: 3,
        field_height: 3,
        win_count: 3,
        first_player: FirstPlayerMode::Host,
    })
}

fn random_bot() -> proto::BotKind {
    proto::BotKind {
        game: Some(proto::bot_kind::Game::Tictactoe(
            proto::TicTacToeBotKind::Random as i32,
        )),
    }
}

/// A view-only strategy: the first free cell, read from the rendered board.
#[derive(Debug)]
struct FirstFreeCell;

impl ViewStrategy for FirstFreeCell {
    fn decide(&mut self, view: &proto::GameView, me: Slot) -> Option<proto::GameCommand> {
        let Some(proto::game_view::Game::Tictactoe(board)) = &view.game else {
            return None;
        };
        if board.current_slot != Some(u32::from(me.get())) {
            return None;
        }
        let free = board.cells.iter().position(|cell| *cell == 0)?;
        let index = u32::try_from(free).ok()?;
        let point = Point {
            x: index % board.field_width,
            y: index / board.field_width,
        };
        Some(TicTacToe::command_to_wire(&Command::Place(point)))
    }
}

fn assert_finished(summary: &MatchSummary) -> &proto::MatchResult {
    assert_eq!(summary.rejected_commands, 0, "{summary:?}");
    summary.result.as_ref().expect("the match has a result")
}

#[tokio::test]
async fn a_hosting_bot_plays_the_servers_bot_to_the_end() {
    within(async {
        let server = TestServer::start().await;
        let mut session = BotSession::connect(BotConfig::new(url(server.address), "secret"))
            .await
            .unwrap();
        let header = session
            .host_match(&LobbySetup {
                name: "sdk".into(),
                settings: small_tictactoe(),
                max_seats: 2,
                built_in_bots: vec![random_bot()],
            })
            .await
            .unwrap();
        let mut minimax = BuiltinBot::<TicTacToe>::new(mg_tictactoe::BotKind::Minimax, 1);
        let summary = session.play_match(header, &mut minimax).await.unwrap();

        assert_eq!(summary.my_slot, Some(Slot::new(0)));
        let result = assert_finished(&summary);
        // Minimax moving first on 3x3 never loses.
        assert!(!result.winner_slots.contains(&1), "{summary:?}");
        assert_eq!(summary.reconnects, 0);
    })
    .await;
}

#[tokio::test]
async fn two_network_bots_play_each_other_in_lockstep() {
    within(async {
        let server = TestServer::start().await;
        let mut alpha = BotSession::connect(BotConfig::new(url(server.address), "secret"))
            .await
            .unwrap();
        let mut beta = BotSession::connect(BotConfig::new(url(server.address), "beta-key"))
            .await
            .unwrap();
        let lobby = alpha
            .create_lobby(&LobbySetup {
                name: "duel".into(),
                settings: small_tictactoe(),
                max_seats: 2,
                built_in_bots: Vec::new(),
            })
            .await
            .unwrap();
        let lobby_id = lobby.lobby_id.clone();
        let (alpha_header, beta_header) =
            tokio::join!(alpha.start_when_ready(lobby), beta.join_match(&lobby_id));
        let (alpha_header, beta_header) = (alpha_header.unwrap(), beta_header.unwrap());
        assert_eq!(alpha_header.match_id, beta_header.match_id);

        let mut view_bot = ViewBot(FirstFreeCell);
        let mut random = BuiltinBot::<TicTacToe>::new(mg_tictactoe::BotKind::Random, 2);
        let (alpha_summary, beta_summary) = tokio::join!(
            alpha.play_match(alpha_header, &mut view_bot),
            beta.play_match(beta_header, &mut random)
        );
        let (alpha_summary, beta_summary) = (alpha_summary.unwrap(), beta_summary.unwrap());
        assert_eq!(alpha_summary.my_slot, Some(Slot::new(0)));
        assert_eq!(beta_summary.my_slot, Some(Slot::new(1)));
        assert_eq!(
            assert_finished(&alpha_summary),
            assert_finished(&beta_summary)
        );
        assert_eq!(alpha_summary.final_tick, beta_summary.final_tick);
        assert_eq!(
            alpha_summary.final_state_hash,
            beta_summary.final_state_hash
        );
    })
    .await;
}

#[tokio::test]
async fn a_dropped_connection_is_resumed_and_the_match_finished() {
    within(async {
        let server = TestServer::start().await;
        let proxy = start_proxy_cutting_first_connection(server.address).await;
        let mut session = BotSession::connect(BotConfig::new(url(proxy), "secret"))
            .await
            .unwrap();
        let header = session
            .host_match(&LobbySetup {
                name: "flaky".into(),
                settings: small_tictactoe(),
                max_seats: 2,
                built_in_bots: vec![random_bot()],
            })
            .await
            .unwrap();
        let mut minimax = BuiltinBot::<TicTacToe>::new(mg_tictactoe::BotKind::Minimax, 3);
        let summary = session.play_match(header, &mut minimax).await.unwrap();

        assert_eq!(summary.reconnects, 1);
        let result = assert_finished(&summary);
        assert!(!result.winner_slots.contains(&1), "{summary:?}");
    })
    .await;
}

/// Forwards TCP to `target`. The first connection is cut just before the first frame with a
/// move reaches the client; later connections pass through untouched.
async fn start_proxy_cutting_first_connection(target: SocketAddr) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let mut cut_next = true;
        loop {
            let (client, _peer) = listener.accept().await.unwrap();
            let server = TcpStream::connect(target).await.unwrap();
            tokio::spawn(relay(client, server, std::mem::take(&mut cut_next)));
        }
    });
    address
}

async fn relay(client: TcpStream, server: TcpStream, cut_at_first_move: bool) {
    let (mut client_read, mut client_write) = client.into_split();
    let (mut server_read, mut server_write) = server.into_split();
    let upstream = tokio::io::copy(&mut client_read, &mut server_write);
    let downstream = async {
        let mut buffer = vec![0_u8; 64 * 1024];
        loop {
            let read = server_read.read(&mut buffer).await?;
            let chunk = &buffer[..read];
            let has_move = chunk.windows(10).any(|window| window == b"\"command\":");
            if read == 0 || (cut_at_first_move && has_move) {
                return Ok::<(), std::io::Error>(());
            }
            client_write.write_all(chunk).await?;
        }
    };
    // Either direction ending ends the relay; dropping the halves closes both sockets.
    tokio::select! {
        _upstream_done = upstream => {}
        _downstream_done = downstream => {}
    }
}
