use std::cmp::Ordering;

/// A column counted from the left edge. Besides the field columns `0..field_width`, `-1` and
/// `field_width` are the off-field columns where cranes enter and leave.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Column(i32);

impl Column {
    pub const fn new(x: i32) -> Self {
        Self(x)
    }

    pub const fn get(self) -> i32 {
        self.0
    }

    /// One column closer to `target`; stays put once there.
    pub(crate) fn step_towards(self, target: Column) -> Self {
        match target.0.cmp(&self.0) {
            Ordering::Greater => Self(self.0.saturating_add(1)),
            Ordering::Less => Self(self.0.saturating_sub(1)),
            Ordering::Equal => self,
        }
    }
}
