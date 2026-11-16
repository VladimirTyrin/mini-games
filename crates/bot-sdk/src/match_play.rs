use std::time::Duration;

use mg_core::{Slot, Tick};
use mg_protocol::proto;
use mg_protocol::proto::client_message::Message as Request;
use mg_protocol::proto::server_message::Message as Reply;
use mg_registry::DynMirror;

use crate::bot_session::BotSession;
use crate::match_summary::MatchSummary;
use crate::sdk_error::SdkError;
use crate::strategy::Strategy;
use crate::submission::Submission;

/// The server refuses catch-ups more often than this per session.
const CATCH_UP_COOLDOWN: Duration = Duration::from_secs(1);

/// Lockstep state of the match being played.
struct MatchPlay<M> {
    match_id: String,
    my_slot: Option<Slot>,
    mirror: M,
    /// The state the strategy was last asked about; each state is decided once.
    decided_at: Option<Tick>,
    /// Frames are useless until the requested (or, after a resume, pushed) catch-up arrives.
    waiting_for_catch_up: bool,
    /// The server refuses commands while a seat is disconnected and during the resume
    /// countdown.
    paused: bool,
    catch_up_request: Option<u32>,
    submission: Submission,
    reconnects_at_start: u32,
    rejected_commands: u32,
}

enum Progress {
    Playing,
    Ended(MatchSummary),
}

impl BotSession {
    /// Plays a match that just started (`header` from `MatchStarted`) to its end. Frames come
    /// from the server; the mirror is rebuilt from a catch-up after a gap, a desync or a resumed
    /// connection.
    pub async fn play_match<S: Strategy>(
        &mut self,
        header: proto::MatchHeader,
        strategy: &mut S,
    ) -> Result<MatchSummary, SdkError> {
        let mut play = MatchPlay {
            my_slot: self.slot_in(&header),
            mirror: strategy.create_mirror(&header).map_err(SdkError::Mirror)?,
            match_id: header.match_id,
            decided_at: None,
            waiting_for_catch_up: false,
            paused: false,
            catch_up_request: None,
            submission: Submission::Settled,
            reconnects_at_start: self.reconnects,
            rejected_commands: 0,
        };
        let mut started = true;
        loop {
            let step = if started {
                started = false;
                self.act(&mut play, strategy)
                    .await
                    .map(|()| Progress::Playing)
            } else {
                self.handle_next(&mut play, strategy).await
            };
            match step {
                Ok(Progress::Playing) => {}
                Ok(Progress::Ended(summary)) => return Ok(summary),
                Err(error) if error.is_connection_loss() => {
                    self.resume().await?;
                    // The server pushes a catch-up when the seat is resumed.
                    play.waiting_for_catch_up = true;
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn handle_next<S: Strategy>(
        &mut self,
        play: &mut MatchPlay<S::Mirror>,
        strategy: &mut S,
    ) -> Result<Progress, SdkError> {
        let message = self.next_message().await?;
        match message.message {
            Some(Reply::Frame(frame)) => self.on_frame(play, strategy, &frame).await?,
            Some(Reply::CatchUp(catch_up)) => self.on_catch_up(play, strategy, catch_up).await?,
            Some(Reply::Ended(ended)) if ended.match_id == play.match_id => {
                return Ok(Progress::Ended(self.summary(
                    play,
                    ended.result,
                    Tick::new(ended.final_tick),
                )));
            }
            // The match ended while the connection was down: the resumed lobby reports it.
            Some(Reply::LobbyState(lobby))
                if play.waiting_for_catch_up && lobby.last_match_id == play.match_id =>
            {
                let final_tick = play.mirror.next_tick();
                return Ok(Progress::Ended(self.summary(
                    play,
                    lobby.last_result,
                    final_tick,
                )));
            }
            Some(Reply::LobbyClosed(_)) => return Err(SdkError::LobbyClosed),
            Some(Reply::Paused(paused)) if paused.match_id == play.match_id => play.paused = true,
            Some(Reply::Resumed(resumed)) if resumed.match_id == play.match_id => {
                self.after_countdown(play, strategy, resumed.countdown_ms)
                    .await?;
            }
            Some(Reply::Error(error)) => {
                self.on_error(play, message.request_id, &error).await?;
                if play.submission
                    == (Submission::AwaitingReply {
                        request_id: message.request_id,
                    })
                {
                    play.submission = Submission::Settled;
                    self.act(play, strategy).await?;
                }
            }
            Some(Reply::Ack(_))
                if play.submission
                    == (Submission::AwaitingReply {
                        request_id: message.request_id,
                    }) =>
            {
                play.submission = Submission::AwaitingFrame;
            }
            _ => {}
        }
        Ok(Progress::Playing)
    }

    async fn on_frame<S: Strategy>(
        &mut self,
        play: &mut MatchPlay<S::Mirror>,
        strategy: &mut S,
        frame: &proto::TickFrame,
    ) -> Result<(), SdkError> {
        let next_tick = play.mirror.next_tick().get();
        if play.waiting_for_catch_up || frame.tick < next_tick {
            return Ok(());
        }
        let in_sync = frame.tick == next_tick && play.mirror.apply_frame(frame).is_ok();
        if !in_sync {
            let local_hash = play.mirror.state_hash();
            let request = Request::ReportDesync(proto::ReportDesync {
                match_id: play.match_id.clone(),
                tick: frame.tick,
                local_hash,
            });
            play.catch_up_request = Some(self.send(request).await?);
            play.waiting_for_catch_up = true;
            return Ok(());
        }
        play.paused = false;
        if play.my_slot.is_some_and(|me| has_command_of(frame, me)) {
            play.submission = Submission::Settled;
        }
        self.act(play, strategy).await
    }

    async fn on_catch_up<S: Strategy>(
        &mut self,
        play: &mut MatchPlay<S::Mirror>,
        strategy: &mut S,
        catch_up: proto::MatchCatchUp,
    ) -> Result<(), SdkError> {
        let Some(header) = catch_up.header else {
            return Ok(());
        };
        if header.match_id != play.match_id {
            return Ok(());
        }
        let mut mirror = strategy.create_mirror(&header).map_err(SdkError::Mirror)?;
        mirror
            .catch_up(catch_up.frames, Tick::new(catch_up.next_tick))
            .map_err(SdkError::CatchUp)?;
        play.mirror = mirror;
        play.waiting_for_catch_up = false;
        play.paused = !catch_up.pauses.is_empty() || catch_up.pause.is_some();
        play.catch_up_request = None;
        // The catch-up shows whether the last command arrived; if the state is still the one it
        // was decided for, it did not, and the strategy is asked again.
        play.decided_at = None;
        play.submission = Submission::Settled;
        if catch_up.resume_countdown_ms > 0 && !play.paused {
            return self
                .after_countdown(play, strategy, catch_up.resume_countdown_ms)
                .await;
        }
        self.act(play, strategy).await
    }

    /// Frames resume after the countdown; a command refused during the pause is decided again.
    async fn after_countdown<S: Strategy>(
        &mut self,
        play: &mut MatchPlay<S::Mirror>,
        strategy: &mut S,
        countdown_ms: u32,
    ) -> Result<(), SdkError> {
        tokio::time::sleep(Duration::from_millis(u64::from(countdown_ms))).await;
        play.paused = false;
        play.decided_at = None;
        self.act(play, strategy).await
    }

    async fn on_error<M>(
        &mut self,
        play: &mut MatchPlay<M>,
        request_id: u32,
        error: &proto::ErrorResponse,
    ) -> Result<(), SdkError> {
        if error.code == proto::ErrorCode::CommandRejected as i32 {
            play.rejected_commands += 1;
        } else if error.code == proto::ErrorCode::RateLimited as i32
            && play.catch_up_request == Some(request_id)
        {
            tokio::time::sleep(CATCH_UP_COOLDOWN).await;
            let request = Request::RequestCatchUp(proto::RequestCatchUp {
                match_id: play.match_id.clone(),
            });
            play.catch_up_request = Some(self.send(request).await?);
        }
        Ok(())
    }

    /// Asks the strategy about the current state once, and submits what it chose.
    async fn act<S: Strategy>(
        &mut self,
        play: &mut MatchPlay<S::Mirror>,
        strategy: &mut S,
    ) -> Result<(), SdkError> {
        let Some(me) = play.my_slot else {
            return Ok(());
        };
        let tick = play.mirror.next_tick();
        if play.waiting_for_catch_up
            || play.paused
            || play.submission != Submission::Settled
            || play.mirror.status().is_finished()
            || play.decided_at == Some(tick)
        {
            return Ok(());
        }
        play.decided_at = Some(tick);
        let Some(command) = strategy.decide(&play.mirror, me) else {
            return Ok(());
        };
        let request = Request::SubmitCommand(proto::SubmitCommand {
            match_id: play.match_id.clone(),
            command: Some(command),
        });
        let request_id = self.send(request).await?;
        play.submission = Submission::AwaitingReply { request_id };
        Ok(())
    }

    fn summary<M: DynMirror>(
        &self,
        play: &MatchPlay<M>,
        result: Option<proto::MatchResult>,
        final_tick: Tick,
    ) -> MatchSummary {
        MatchSummary {
            match_id: play.match_id.clone(),
            my_slot: play.my_slot,
            result,
            final_tick,
            final_state_hash: play.mirror.state_hash(),
            reconnects: self.reconnects - play.reconnects_at_start,
            rejected_commands: play.rejected_commands,
        }
    }
}

fn has_command_of(frame: &proto::TickFrame, me: Slot) -> bool {
    frame.inputs.iter().any(|input| {
        matches!(
            &input.input,
            Some(proto::frame_input::Input::Command(command)) if command.slot == u32::from(me.get())
        )
    })
}
