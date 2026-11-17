/// Whether a capture, once begun, must take every piece it can.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum CaptureRule {
    /// Russian draughts: a capture goes on while it can, and a king lands where it can capture
    /// again when there is such a square.
    #[default]
    ToTheEnd,
    /// A house rule: the move may end after any jump, and a king may land on any square beyond
    /// the piece it takes.
    MayStop,
}
