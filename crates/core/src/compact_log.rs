use crate::compact_log_entries::CompactLogEntries;
use crate::compact_log_error::CompactLogError;
use crate::hash_policy::HashPolicy;
use crate::logged_frame::LoggedFrame;
use crate::tick::Tick;

/// Frames `0..end` of a match with empty frames omitted (see
/// [`crate::Frame::belongs_in_compact_log`]).
///
/// Logs arrive from untrusted peers (catch-ups, replay files), so construction checks that an
/// authority could have produced the log before anything is expanded from it: ticks strictly
/// increase and stay below `end`, and at most `every_frames - 1` frames in a row are omitted.
/// The bound holds because the authority hashes the frame of every tick `t` with
/// `(t + 1) % every_frames == 0`, so each window of `every_frames` ticks has a logged frame;
/// it also covers the tail, so `end <= last.tick + every_frames` (`end < every_frames` for an
/// empty log). A receiver therefore synthesizes at most `(frames + 1) * (every_frames - 1)`
/// empty frames however large `end` claims to be.
#[derive(Clone, Debug)]
pub struct CompactLog<F> {
    frames: Vec<F>,
    end: Tick,
}

impl<F: LoggedFrame> CompactLog<F> {
    /// `policy` must be the one the authority sealed with: [`HashPolicy::for_clock`] of the game.
    pub fn new(frames: Vec<F>, end: Tick, policy: HashPolicy) -> Result<Self, CompactLogError> {
        Self::validate(&frames, end, policy)?;
        Ok(Self { frames, end })
    }

    /// The checks of [`Self::new`] without taking the frames.
    pub fn validate(frames: &[F], end: Tick, policy: HashPolicy) -> Result<(), CompactLogError> {
        let max_omitted_frames = policy.every_frames().saturating_sub(1);
        let check_gap = |from: Tick, to: Tick| {
            if to.get().saturating_sub(from.get()) > max_omitted_frames {
                Err(CompactLogError::GapTooLong {
                    from,
                    to,
                    max_omitted_frames,
                })
            } else {
                Ok(())
            }
        };
        let mut previous: Option<Tick> = None;
        let mut first_uncovered = Tick::ZERO;
        for tick in frames.iter().map(LoggedFrame::tick) {
            if let Some(previous) = previous
                && tick <= previous
            {
                return Err(CompactLogError::TicksNotIncreasing {
                    previous,
                    next: tick,
                });
            }
            if tick >= end {
                return Err(CompactLogError::FrameAtOrAfterEnd { tick, end });
            }
            check_gap(first_uncovered, tick)?;
            previous = Some(tick);
            first_uncovered = tick.next();
        }
        check_gap(first_uncovered, end)
    }

    pub fn end(&self) -> Tick {
        self.end
    }

    pub fn frames(&self) -> &[F] {
        &self.frames
    }

    /// Entries for ticks `from..end`.
    pub fn entries_from(&self, from: Tick) -> CompactLogEntries<'_, F> {
        let first = self.frames.partition_point(|frame| frame.tick() < from);
        let frames = self.frames.get(first..).unwrap_or_default();
        CompactLogEntries::new(frames, from, self.end)
    }
}
