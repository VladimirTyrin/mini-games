use mg_core::{Authority, Awaiting, BotSeat, Clock, HasBots, PlayerSpec, Slot, Status, Tick};
use mg_protocol::{WireBots, WireGame, frame_to_wire, proto};

use crate::game_kind::GameKind;
use crate::registry_error::{CreateError, SubmitError};
use crate::seat_spec::SeatSpec;

/// [`Authority`] with the game type erased; speaks wire types.
pub trait DynAuthority: Send + std::fmt::Debug {
    fn kind(&self) -> GameKind;
    fn clock(&self) -> Clock;
    fn submit(&mut self, slot: Slot, command: &proto::GameCommand) -> Result<(), SubmitError>;
    fn drop_player(&mut self, slot: Slot);
    fn run_bots(&mut self);
    fn is_waiting_for_bot(&self) -> bool;
    fn has_pending_inputs(&self) -> bool;
    fn seal_frame(&mut self) -> proto::TickFrame;
    fn awaiting(&self) -> Awaiting;
    fn status(&self) -> Status;
    fn next_tick(&self) -> Tick;
    fn compact_log(&self) -> Vec<proto::TickFrame>;
}

#[derive(Debug)]
struct AuthorityAdapter<G: WireGame> {
    kind: GameKind,
    authority: Authority<G>,
}

impl<G: WireGame> DynAuthority for AuthorityAdapter<G> {
    fn kind(&self) -> GameKind {
        self.kind
    }

    fn clock(&self) -> Clock {
        self.authority.game().clock()
    }

    fn submit(&mut self, slot: Slot, command: &proto::GameCommand) -> Result<(), SubmitError> {
        let command = G::command_from_wire(command)?;
        Ok(self.authority.submit(slot, command)?)
    }

    fn drop_player(&mut self, slot: Slot) {
        self.authority.drop_player(slot);
    }

    fn run_bots(&mut self) {
        self.authority.run_bots();
    }

    fn is_waiting_for_bot(&self) -> bool {
        self.authority.is_waiting_for_bot()
    }

    fn has_pending_inputs(&self) -> bool {
        self.authority.has_pending_inputs()
    }

    fn seal_frame(&mut self) -> proto::TickFrame {
        frame_to_wire::<G>(&self.authority.seal_frame())
    }

    fn awaiting(&self) -> Awaiting {
        self.authority.game().awaiting()
    }

    fn status(&self) -> Status {
        self.authority.game().status()
    }

    fn next_tick(&self) -> Tick {
        self.authority.next_tick()
    }

    fn compact_log(&self) -> Vec<proto::TickFrame> {
        self.authority
            .compact_log()
            .iter()
            .map(frame_to_wire::<G>)
            .collect()
    }
}

fn player_specs(seats: &[SeatSpec]) -> Vec<PlayerSpec> {
    seats.iter().map(SeatSpec::player_spec).collect()
}

pub(crate) fn with_bots<G: WireBots>(
    kind: GameKind,
    settings: &proto::GameSettings,
    seats: &[SeatSpec],
    seed: u64,
) -> Result<Box<dyn DynAuthority>, CreateError> {
    let settings = G::settings_from_wire(settings)?;
    let mut bots = Vec::new();
    for (slot, seat) in Slot::all(seats.len()).zip(seats) {
        if let SeatSpec::Bot(wire_kind) = seat {
            let bot_kind = G::bot_kind_from_wire(wire_kind)?;
            bots.push(BotSeat {
                slot,
                bot: <G as HasBots>::make_bot(bot_kind),
                budget_nodes: <G as HasBots>::default_bot_budget(bot_kind),
            });
        }
    }
    let authority = Authority::<G>::new(&settings, &player_specs(seats), seed, bots)?;
    Ok(Box::new(AuthorityAdapter { kind, authority }))
}

pub(crate) fn without_bots<G: WireGame>(
    kind: GameKind,
    settings: &proto::GameSettings,
    seats: &[SeatSpec],
    seed: u64,
) -> Result<Box<dyn DynAuthority>, CreateError> {
    if seats.iter().any(|seat| matches!(seat, SeatSpec::Bot(_))) {
        return Err(CreateError::NoBots(G::INFO.name));
    }
    let settings = G::settings_from_wire(settings)?;
    let authority = Authority::<G>::new(&settings, &player_specs(seats), seed, Vec::new())?;
    Ok(Box::new(AuthorityAdapter { kind, authority }))
}
