//! Canonical state hashing for lockstep desync detection.
//!
//! Games feed their state field-by-field in a fixed order. The hash must not depend on
//! iteration order of unordered containers, pointer values or platform word size.

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64-bit hasher with explicit, little-endian, fixed-width writes.
#[derive(Clone, Debug)]
pub struct StateHasher {
    state: u64,
}

impl Default for StateHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHasher {
    /// Fresh hasher.
    pub fn new() -> Self {
        Self { state: FNV_OFFSET }
    }

    /// Feeds raw bytes.
    pub fn bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.state ^= u64::from(b);
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
    }

    /// Feeds a `u8`.
    pub fn u8(&mut self, v: u8) {
        self.bytes(&[v]);
    }

    /// Feeds a `bool` as one byte.
    pub fn bool(&mut self, v: bool) {
        self.u8(u8::from(v));
    }

    /// Feeds a `u16`.
    pub fn u16(&mut self, v: u16) {
        self.bytes(&v.to_le_bytes());
    }

    /// Feeds a `u32`.
    pub fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }

    /// Feeds an `i32`.
    pub fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }

    /// Feeds a `u64`.
    pub fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }

    /// Feeds an `i64`.
    pub fn i64(&mut self, v: i64) {
        self.bytes(&v.to_le_bytes());
    }

    /// Feeds a length/count. Always 8 bytes so 32- and 64-bit targets agree.
    pub fn len(&mut self, v: usize) {
        self.u64(v as u64);
    }

    /// Final value.
    pub fn finish(&self) -> u64 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector() {
        let mut h = StateHasher::new();
        h.bytes(b"a");
        assert_eq!(h.finish(), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn order_matters() {
        let mut a = StateHasher::new();
        a.u32(1);
        a.u32(2);
        let mut b = StateHasher::new();
        b.u32(2);
        b.u32(1);
        assert_ne!(a.finish(), b.finish());
    }
}
