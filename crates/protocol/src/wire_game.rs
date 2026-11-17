use mg_core::{Game, GameSettings, SeatSecret, SecretCommand};

use crate::proto;
use crate::wire_error::WireError;

/// The unchecked settings form a game parses ([`GameSettings::parse`]).
pub type RawSettingsOf<G> = <<G as Game>::Settings as GameSettings>::Raw;

/// Conversions between a game's domain types and the `minigames.v2` envelopes.
///
/// `*_from_wire` checks the shape (the right game, required fields, known enum values, integers
/// that fit) and never clamps or panics; game rules, setting ranges included, stay with the game.
pub trait WireGame: Game {
    /// Raw settings: the caller parses them with [`GameSettings::parse`], the one place ranges
    /// are enforced.
    fn settings_from_wire(settings: &proto::GameSettings)
    -> Result<RawSettingsOf<Self>, WireError>;
    fn settings_to_wire(settings: &RawSettingsOf<Self>) -> proto::GameSettings;

    fn command_from_wire(command: &proto::GameCommand) -> Result<Self::Command, WireError>;
    fn command_to_wire(command: &Self::Command) -> proto::GameCommand;

    fn world_event_from_wire(event: &proto::GameWorldEvent) -> Result<Self::WorldEvent, WireError>;
    fn world_event_to_wire(event: &Self::WorldEvent) -> proto::GameWorldEvent;

    fn view_to_wire(view: &Self::View) -> proto::GameView;

    /// Games that hide nothing take no private commands: every one is from another game.
    fn secret_command_from_wire(
        _command: &proto::GameSecretCommand,
    ) -> Result<SecretCommand<Self>, WireError> {
        Err(WireError::WrongGame)
    }

    fn secret_command_to_wire(_command: &SecretCommand<Self>) -> proto::GameSecretCommand {
        proto::GameSecretCommand { game: None }
    }

    fn seat_secret_to_wire(_secret: &SeatSecret<Self>) -> proto::GameSeatSecret {
        proto::GameSeatSecret { game: None }
    }
}

/// Built-in bot kinds on the wire, for games that have bots.
pub trait WireBots: WireGame + mg_core::HasBots {
    fn bot_kind_from_wire(kind: &proto::BotKind) -> Result<Self::BotKind, WireError>;
    fn bot_kind_to_wire(kind: Self::BotKind) -> proto::BotKind;
}
