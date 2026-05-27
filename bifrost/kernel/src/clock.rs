//! [`SequencedInstant`] — deterministic logical clock.
//!
//! **Rule 5 — No `SystemTime::now()`.**
//!
//! All time-based ordering in the BIFROST system uses `SequencedInstant`.
//! Wall-clock time (`SystemTime`, `Instant`, Unix milliseconds) must **never**
//! be used for ordering, causality, or simulation state.
//!
//! Wall-clock timestamps are allowed in **informational / audit** fields only,
//! and must be explicitly annotated as such.  They must never appear in hash
//! inputs or reducer logic.
//!
//! ## Ordering semantics
//!
//! ```text
//! SequencedInstant { tick: 5, seq: 3 }  <  SequencedInstant { tick: 5, seq: 4 }
//! SequencedInstant { tick: 4, seq: 999 } < SequencedInstant { tick: 5, seq: 0 }
//! ```
//!
//! Two instants with the same `(tick, seq)` are equal — the system guarantees
//! no two events share the same `(tick, seq)` within a zone.

use serde::{Deserialize, Serialize};

/// Tick-sequence logical clock position.
///
/// Replaces `SystemTime::now()` and Unix-ms timestamps for all ordering.
/// Constructed by the simulation clock from the current `LockstepTick` and
/// an atomically-incremented per-tick sub-sequence counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SequencedInstant {
    /// World simulation tick at time of event.
    pub tick: u64,
    /// Monotonically-increasing sub-tick sequence.
    ///
    /// Unique within a `(zone_id, tick)` pair.  Allows total ordering of
    /// events that occur within the same tick.
    pub seq: u64,
}

impl SequencedInstant {
    /// Construct from explicit components.
    ///
    /// Callers must ensure `seq` is unique within `(zone, tick)`.
    pub const fn new(tick: u64, seq: u64) -> Self {
        Self { tick, seq }
    }

    /// The genesis instant — before any simulation tick has run.
    pub const ZERO: Self = Self { tick: 0, seq: 0 };

    /// Advance the sub-sequence counter within the same tick.
    #[must_use]
    pub fn next_seq(self) -> Self {
        Self { tick: self.tick, seq: self.seq + 1 }
    }

    /// Advance to the first instant of the next tick.
    #[must_use]
    pub fn next_tick(self) -> Self {
        Self { tick: self.tick + 1, seq: 0 }
    }
}

impl std::fmt::Display for SequencedInstant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "t{}:s{}", self.tick, self.seq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_within_tick() {
        let a = SequencedInstant::new(5, 3);
        let b = SequencedInstant::new(5, 4);
        assert!(a < b);
    }

    #[test]
    fn tick_dominates_seq() {
        let a = SequencedInstant::new(4, 999);
        let b = SequencedInstant::new(5, 0);
        assert!(a < b);
    }

    #[test]
    fn next_seq_increments() {
        let a = SequencedInstant::new(3, 7);
        assert_eq!(a.next_seq(), SequencedInstant::new(3, 8));
    }

    #[test]
    fn next_tick_resets_seq() {
        let a = SequencedInstant::new(3, 42);
        assert_eq!(a.next_tick(), SequencedInstant::new(4, 0));
    }

    #[test]
    fn display() {
        assert_eq!(SequencedInstant::new(10, 3).to_string(), "t10:s3");
    }

    #[test]
    fn serde_round_trip() {
        let a = SequencedInstant::new(7, 2);
        let json = serde_json::to_string(&a).unwrap();
        let b: SequencedInstant = serde_json::from_str(&json).unwrap();
        assert_eq!(a, b);
    }
}
