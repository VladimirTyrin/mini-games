/// Work limit for one bot decision in abstract nodes (positions evaluated, cells scanned).
/// Work instead of wall time keeps bots deterministic and equally strong on every platform.
#[derive(Clone, Debug)]
pub struct Budget {
    remaining: u64,
}

impl Budget {
    pub fn new(nodes: u64) -> Self {
        Self { remaining: nodes }
    }

    /// Returns `false` once exhausted; the bot should then return its best move so far.
    pub fn spend(&mut self, nodes: u64) -> bool {
        if self.remaining >= nodes {
            self.remaining -= nodes;
            true
        } else {
            self.remaining = 0;
            false
        }
    }

    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    pub fn is_exhausted(&self) -> bool {
        self.remaining == 0
    }
}
