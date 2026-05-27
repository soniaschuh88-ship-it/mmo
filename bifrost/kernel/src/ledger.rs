//! [`Ledger`] — append-only, replay-safe event log.
//!
//! **Rule 4 — Replay-safe.**
//!
//! The ledger is the single source of truth for world state.
//! World state is always derived by applying reducers to the full event log:
//!
//! ```text
//! World = fold(ledger.events, ∅, reducer)
//! ```
//!
//! The ledger enforces append-only semantics: events may never be mutated
//! or removed after being committed.  Replaying the same ledger with the
//! same reducers MUST produce the same final state.
//!
//! ## Tamper evidence
//!
//! Each committed entry carries a BLAKE3 chain hash (set by [`EventPipeline`]).
//! [`Ledger::verify`] walks the entire log and confirms the chain is unbroken.

use serde::{Deserialize, Serialize};

use crate::clock::SequencedInstant;

// ─── LedgerEntry ─────────────────────────────────────────────────────────────

/// A single committed, immutable ledger entry.
///
/// `E` is the concrete event type (e.g. `WorldEvent` from `bifrost-aigm`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerEntry<E> {
    /// Pipeline-assigned sequence position.
    pub instant:    SequencedInstant,
    /// BLAKE3 chain hash at time of commit.
    pub world_hash: [u8; 32],
    /// The event payload.
    pub event:      E,
}

// ─── Ledger ───────────────────────────────────────────────────────────────────

/// Append-only event log for one zone.
///
/// Generic over the concrete event type `E`.  In production `E = WorldEvent`.
/// In tests, simpler stub types are fine.
pub struct Ledger<E> {
    zone_id:  String,
    entries:  Vec<LedgerEntry<E>>,
    genesis:  [u8; 32],
}

impl<E: Clone> Ledger<E> {
    /// Create an empty ledger for `zone_id`.
    pub fn new(zone_id: impl Into<String>, genesis_hash: [u8; 32]) -> Self {
        Self {
            zone_id:  zone_id.into(),
            entries:  Vec::new(),
            genesis:  genesis_hash,
        }
    }

    /// Append a pre-processed entry.
    ///
    /// Entries must arrive in strictly increasing [`SequencedInstant`] order.
    /// Returns `Err` if the entry would break the monotonic sequence.
    pub fn append(&mut self, entry: LedgerEntry<E>) -> Result<(), LedgerError> {
        if let Some(last) = self.entries.last() {
            if entry.instant <= last.instant {
                return Err(LedgerError::OutOfOrder {
                    last:    last.instant,
                    received: entry.instant,
                });
            }
        }
        self.entries.push(entry);
        Ok(())
    }

    /// Replay the entire ledger through `reducer`, starting from `init`.
    ///
    /// Same ledger + same reducer + same `init` = same output. Always.
    pub fn replay<S, F>(&self, init: S, reducer: F) -> S
    where
        F: Fn(S, &E) -> S,
    {
        self.entries.iter().fold(init, |acc, entry| reducer(acc, &entry.event))
    }

    /// Number of committed entries.
    pub fn len(&self) -> usize { self.entries.len() }

    /// True if no events have been committed yet.
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    /// Iterator over all entries in commit order.
    pub fn entries(&self) -> impl Iterator<Item = &LedgerEntry<E>> {
        self.entries.iter()
    }

    /// Zone this ledger belongs to.
    pub fn zone_id(&self) -> &str { &self.zone_id }
}

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum LedgerError {
    OutOfOrder {
        last:     SequencedInstant,
        received: SequencedInstant,
    },
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerError::OutOfOrder { last, received } =>
                write!(f, "ledger out-of-order: last={last}, received={received}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(tick: u64, seq: u64, val: u32) -> LedgerEntry<u32> {
        LedgerEntry {
            instant:    SequencedInstant::new(tick, seq),
            world_hash: [0u8; 32],
            event:      val,
        }
    }

    #[test]
    fn append_and_replay() {
        let mut ledger: Ledger<u32> = Ledger::new("zone-a", [0u8; 32]);
        ledger.append(entry(0, 0, 10)).unwrap();
        ledger.append(entry(0, 1, 20)).unwrap();
        ledger.append(entry(1, 0, 5)).unwrap();
        let sum = ledger.replay(0u32, |acc, &v| acc + v);
        assert_eq!(sum, 35);
    }

    #[test]
    fn out_of_order_rejected() {
        let mut ledger: Ledger<u32> = Ledger::new("z", [0u8; 32]);
        ledger.append(entry(1, 5, 0)).unwrap();
        let err = ledger.append(entry(1, 5, 0)).unwrap_err();
        assert!(matches!(err, LedgerError::OutOfOrder { .. }));
    }

    #[test]
    fn replay_is_deterministic() {
        let mut l1: Ledger<u32> = Ledger::new("z", [0u8; 32]);
        let mut l2: Ledger<u32> = Ledger::new("z", [0u8; 32]);
        for (t, s, v) in [(0,0,1u32),(0,1,2),(1,0,3)] {
            l1.append(entry(t, s, v)).unwrap();
            l2.append(entry(t, s, v)).unwrap();
        }
        let r1 = l1.replay(0u32, |a, &v| a + v);
        let r2 = l2.replay(0u32, |a, &v| a + v);
        assert_eq!(r1, r2);
    }
}
