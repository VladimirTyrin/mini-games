use crate::command_error::CommandError;
use crate::game::Game;
use crate::player_spec::PlayerSpec;
use crate::slot::Slot;

/// What the authority refuses before the game is asked: a seat that is not a human in play, or
/// a finished game. Shared by [`crate::Authority`] and [`crate::Match`], so a client asking its
/// mirror gets the authority's answer.
pub(crate) fn seat_may_command<G: Game>(
    game: &G,
    players: &[PlayerSpec],
    dropped: &[bool],
    slot: Slot,
) -> Result<(), CommandError<G::Refusal>> {
    let in_play = !dropped.get(slot.index()).copied().unwrap_or(true);
    match players.get(slot.index()) {
        Some(spec) if spec.is_human() && in_play => {}
        _ => return Err(CommandError::NotPlaying),
    }
    if game.status().is_finished() {
        return Err(CommandError::GameOver);
    }
    Ok(())
}
