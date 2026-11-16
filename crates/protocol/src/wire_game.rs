use mg_core::Game;

use crate::proto;
use crate::wire_error::WireError;

/// Conversions between a game's domain types and the `minigames.v2` envelopes.
///
/// `*_from_wire` must validate everything (ranges, enum values) but leave game-rule validation
/// to the game itself.
pub trait WireGame: Game {
    fn settings_from_wire(settings: &proto::GameSettings) -> Result<Self::Settings, WireError>;
    fn settings_to_wire(settings: &Self::Settings) -> proto::GameSettings;

    fn command_from_wire(command: &proto::GameCommand) -> Result<Self::Command, WireError>;
    fn command_to_wire(command: &Self::Command) -> proto::GameCommand;

    fn world_event_from_wire(event: &proto::GameWorldEvent) -> Result<Self::WorldEvent, WireError>;
    fn world_event_to_wire(event: &Self::WorldEvent) -> proto::GameWorldEvent;

    fn view_to_wire(view: &Self::View) -> proto::GameView;
}

/// Built-in bot kinds on the wire, for games that have bots.
pub trait WireBots: WireGame + mg_core::HasBots {
    fn bot_kind_from_wire(kind: &proto::BotKind) -> Result<Self::BotKind, WireError>;
    fn bot_kind_to_wire(kind: Self::BotKind) -> proto::BotKind;
}
