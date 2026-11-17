use mg_core::{
    Authority, Awaiting, BotSeat, Clock, DecidedBotTurn, HasBots, PlayerSpec, Slot, Status, Tick,
};
use mg_protocol::{WireBots, WireGame, frame_to_wire, proto};

use crate::parse_settings;

use crate::create_error::CreateError;
use crate::dyn_authority::DynAuthority;
use crate::dyn_bot_decision::DynBotDecision;
use crate::dyn_bot_turn::DynBotTurn;
use crate::game_kind::GameKind;
use crate::seat_spec::SeatSpec;
use crate::submit_error::SubmitError;
use crate::typed_bot_turn::TypedBotTurn;

#[derive(Debug)]
pub(crate) struct AuthorityAdapter<G: WireGame> {
    kind: GameKind,
    authority: Authority<G>,
    wire_log: Vec<proto::TickFrame>,
}

impl<G: WireGame> AuthorityAdapter<G> {
    fn boxed(kind: GameKind, mut authority: Authority<G>) -> Box<dyn DynAuthority> {
        authority.discard_compact_log();
        Box::new(Self {
            kind,
            authority,
            wire_log: Vec::new(),
        })
    }
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

    fn begin_bot_turn(&mut self) -> Option<Box<dyn DynBotTurn>> {
        self.authority
            .begin_bot_turn()
            .map(|turn| Box::new(TypedBotTurn(turn)) as Box<dyn DynBotTurn>)
    }

    fn finish_bot_turn(&mut self, decision: DynBotDecision) {
        match decision.downcast::<DecidedBotTurn<G>>() {
            Some(decided) => self.authority.finish_bot_turn(decided),
            None => debug_assert!(false, "a bot decision from another game"),
        }
    }

    fn has_pending_inputs(&self) -> bool {
        self.authority.has_pending_inputs()
    }

    fn seal_frame(&mut self) -> proto::TickFrame {
        let frame = self.authority.seal_frame();
        let wire = frame_to_wire::<G>(&frame);
        if frame.belongs_in_compact_log() {
            self.wire_log.push(wire.clone());
        }
        wire
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

    fn compact_log(&self) -> &[proto::TickFrame] {
        &self.wire_log
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
    let settings = parse_settings::<G>(settings)?;
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
    Ok(AuthorityAdapter::boxed(kind, authority))
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
    let settings = parse_settings::<G>(settings)?;
    let authority = Authority::<G>::new(&settings, &player_specs(seats), seed, Vec::new())?;
    Ok(AuthorityAdapter::boxed(kind, authority))
}
