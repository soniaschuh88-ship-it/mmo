//! LockstepTick — monotonically increasing world tick counter.

use serde::{Deserialize, Serialize};

/// The global world tick number.
///
/// A tick is the atomic unit of simulation time. All state transitions happen
/// exactly at tick boundaries. The lockstep protocol ensures every peer
/// completes tick N before any peer begins tick N+1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct LockstepTick(pub u64);

impl LockstepTick {
    pub fn zero() -> Self {
        Self(0)
    }

    /// The tick immediately following this one.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }

    /// True if this tick is strictly before `other`.
    pub fn is_before(self, other: Self) -> bool {
        self.0 < other.0
    }

    /// True if this tick is behind by more than `lag_limit` ticks.
    pub fn is_lagging(self, current: Self, lag_limit: u64) -> bool {
        current.0.saturating_sub(self.0) > lag_limit
    }
}

impl std::fmt::Display for LockstepTick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tick({})", self.0)
    }
}

impl From<u64> for LockstepTick {
    fn from(n: u64) -> Self {
        Self(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering() {
        assert!(LockstepTick(0) < LockstepTick(1));
        assert!(LockstepTick(999) < LockstepTick(1000));
    }

    #[test]
    fn next_advances() {
        let t = LockstepTick(42);
        assert_eq!(t.next(), LockstepTick(43));
    }

    #[test]
    fn lagging() {
        let current = LockstepTick(100);
        assert!(LockstepTick(90).is_lagging(current, 5));
        assert!(!LockstepTick(97).is_lagging(current, 5));
    }
}
