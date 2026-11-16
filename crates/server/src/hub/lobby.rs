use std::collections::VecDeque;

use mg_protocol::proto;
use mg_registry::GameKind;
use tokio::time::Instant;

use super::finished_match::FinishedMatch;
use super::lobby_phase::LobbyPhase;
use super::lobby_seat::LobbySeat;
use super::stored_replay::StoredReplay;
use crate::ids::{LobbyId, SessionId};
use crate::limits::Limits;

#[derive(Debug)]
pub(super) struct Lobby {
    pub id: LobbyId,
    pub name: String,
    pub host: SessionId,
    pub kind: GameKind,
    pub settings: proto::GameSettings,
    pub max_seats: u8,
    /// Seat order becomes slot order; the host's seat is kept first so "host moves first"
    /// settings refer to slot 0.
    pub seats: Vec<LobbySeat>,
    pub observers: Vec<SessionId>,
    pub phase: LobbyPhase,
    pub last_activity: Instant,
    pub replays: VecDeque<StoredReplay>,
    pub last_match: Option<FinishedMatch>,
    pub next_bot_number: u32,
}

impl Lobby {
    pub(super) fn members(&self) -> impl Iterator<Item = &SessionId> {
        self.seats
            .iter()
            .filter_map(LobbySeat::session)
            .chain(self.observers.iter())
    }

    pub(super) fn seat_index(&self, session: &SessionId) -> Option<usize> {
        self.seats
            .iter()
            .position(|seat| seat.session() == Some(session))
    }

    pub(super) fn seated_humans(&self) -> impl Iterator<Item = &SessionId> {
        self.seats.iter().filter_map(LobbySeat::session)
    }

    /// Removes the session from seats and observers; returns whether it was a member.
    pub(super) fn remove_member(&mut self, session: &SessionId) -> bool {
        let before = self.seats.len() + self.observers.len();
        self.seats.retain(|seat| seat.session() != Some(session));
        self.observers.retain(|observer| observer != session);
        before != self.seats.len() + self.observers.len()
    }

    pub(super) fn move_host_seat_first(&mut self) {
        if let Some(index) = self.seat_index(&self.host.clone())
            && index > 0
        {
            let seat = self.seats.remove(index);
            self.seats.insert(0, seat);
        }
    }

    /// Every seated human voted to play again (and at least one is seated).
    pub(super) fn everyone_voted_to_play_again(&self) -> bool {
        let LobbyPhase::PostMatch { play_again_votes } = &self.phase else {
            return false;
        };
        let mut humans = self.seated_humans().peekable();
        humans.peek().is_some() && humans.all(|human| play_again_votes.contains(human))
    }

    pub(super) fn reset_ready(&mut self) {
        for seat in &mut self.seats {
            if let LobbySeat::Human { ready, .. } = seat {
                *ready = false;
            }
        }
    }

    pub(super) fn store_replay(&mut self, replay: StoredReplay) {
        if self.replays.len() >= Limits::REPLAYS_PER_LOBBY {
            self.replays.pop_front();
        }
        self.replays.push_back(replay);
    }
}
