//! LockstepTick — zone-local simulation clock.
//!
//! # Why not a global u64?
//!
//! A single global `seq: u64` is a single-writer bottleneck. Under multi-zone
//! sharding (the target architecture), each zone runs its own simulation loop.
//! Cross-region latency would force every zone to sync with a central counter,
//! killing horizontal scalability.
//!
//! # Design
//!
//! Each tick is scoped to a `ZoneId`:
//!
//! ```text
//! LockstepTick { zone_id: ZoneId(3), local_seq: 4412, epoch: 7 }
//! ```
//!
//! - `zone_id`   — which spatial partition owns this tick
//! - `local_seq` — monotonically increasing within the zone (the real clock)
//! - `epoch`     — global epoch counter, set by DELPHOS authority; used for
//!                 audit and cross-zone reconciliation ONLY, never for ordering
//!
//! # Ordering rules
//!
//! - **Same zone**: compare by `local_seq`. Fast, deterministic.
//! - **Cross zone**: use `causal_cmp()` → `CausalOrder`. Never assume `<` across zones.

use serde::{Deserialize, Serialize};

// ─── ZoneId ───────────────────────────────────────────────────────────────────

/// Identifies a spatial simulation partition.
///
/// Each zone runs an independent deterministic tick loop. Cross-zone events
/// require causal ordering, not sequence ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
         Serialize, Deserialize, Default)]
pub struct ZoneId(pub u32);

impl ZoneId {
    /// The "global" zone — used before zone partitioning is established,
    /// and for single-zone deployments.
    pub const GLOBAL: Self = Self(0);

    pub fn new(id: u32) -> Self { Self(id) }
}

impl std::fmt::Display for ZoneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "zone({})", self.0)
    }
}

// ─── CausalOrder ──────────────────────────────────────────────────────────────

/// Result of comparing two ticks across zone boundaries.
///
/// Within a zone, use normal `<`/`>` ordering on `local_seq`.
/// Across zones, use `causal_cmp()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalOrder {
    /// `self` happened strictly before `other`.
    Before,
    /// `self` happened strictly after `other`.
    After,
    /// Same tick in the same zone.
    Equal,
    /// Different zones with no established causal relationship.
    /// The events are concurrent — neither happened before the other.
    Concurrent,
}

// ─── LockstepTick ─────────────────────────────────────────────────────────────

/// A zone-scoped simulation tick.
///
/// # Ordering
///
/// `Ord` is defined as `(zone_id, local_seq)` — valid for `BTreeMap` keying
/// and same-zone comparisons. For cross-zone ordering call `causal_cmp()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LockstepTick {
    /// The zone this tick belongs to.
    pub zone_id:   ZoneId,
    /// Monotonically increasing within the zone — the primary simulation clock.
    pub local_seq: u64,
    /// Global epoch, set by DELPHOS. Audit/reconciliation only.
    /// Do NOT use for simulation ordering.
    pub epoch:     u64,
}

impl LockstepTick {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Tick 0 in the global zone (backward-compatible default).
    pub fn zero() -> Self {
        Self { zone_id: ZoneId::GLOBAL, local_seq: 0, epoch: 0 }
    }

    /// First tick for a specific zone at the given epoch.
    pub fn zone_start(zone_id: ZoneId, epoch: u64) -> Self {
        Self { zone_id, local_seq: 0, epoch }
    }

    /// Build a tick at an explicit position (used in tests and snapshots).
    pub fn at(zone_id: ZoneId, local_seq: u64, epoch: u64) -> Self {
        Self { zone_id, local_seq, epoch }
    }

    /// Convert a legacy `u64` sequence number into a global-zone tick.
    ///
    /// Used for API backward compat. New code should use `zone_start` / `at`.
    pub fn from_legacy(seq: u64) -> Self {
        Self { zone_id: ZoneId::GLOBAL, local_seq: seq, epoch: seq }
    }

    // ── Advancement ───────────────────────────────────────────────────────────

    /// Next tick within the same zone (increments `local_seq` only).
    /// The `epoch` is never changed by the tick loop; only DELPHOS sets it.
    pub fn next(self) -> Self {
        Self { zone_id: self.zone_id, local_seq: self.local_seq + 1, epoch: self.epoch }
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    pub fn zone_id(self)   -> ZoneId { self.zone_id }
    pub fn local_seq(self) -> u64    { self.local_seq }
    pub fn epoch(self)     -> u64    { self.epoch }

    // ── Zone predicates ───────────────────────────────────────────────────────

    pub fn same_zone(self, other: Self) -> bool {
        self.zone_id == other.zone_id
    }

    /// Within-zone ordering. Panics if the ticks belong to different zones.
    /// Use `causal_cmp` when zone membership is uncertain.
    pub fn zone_cmp(self, other: Self) -> std::cmp::Ordering {
        assert!(
            self.same_zone(other),
            "zone_cmp called on ticks from different zones: {} vs {}",
            self.zone_id, other.zone_id,
        );
        self.local_seq.cmp(&other.local_seq)
    }

    /// Cross-zone causal comparison.
    pub fn causal_cmp(self, other: Self) -> CausalOrder {
        if self.zone_id != other.zone_id {
            // Without a full vector clock we classify cross-zone as concurrent.
            // Phase 2 will replace this with vector-clock inference.
            CausalOrder::Concurrent
        } else {
            match self.local_seq.cmp(&other.local_seq) {
                std::cmp::Ordering::Less    => CausalOrder::Before,
                std::cmp::Ordering::Greater => CausalOrder::After,
                std::cmp::Ordering::Equal   => CausalOrder::Equal,
            }
        }
    }

    /// True if this tick is strictly before `other` within the same zone.
    pub fn is_before(self, other: Self) -> bool {
        self.same_zone(other) && self.local_seq < other.local_seq
    }

    /// True if this tick is lagging behind `current` by more than `lag_limit`.
    /// Only meaningful within the same zone.
    pub fn is_lagging(self, current: Self, lag_limit: u64) -> bool {
        self.same_zone(current)
            && current.local_seq.saturating_sub(self.local_seq) > lag_limit
    }

    // ── Hashing ───────────────────────────────────────────────────────────────

    /// Canonical 16-byte LE representation for BLAKE3 inputs.
    ///
    /// Layout: zone_id_le4 || local_seq_le8 || epoch_le4 (truncated to 4)
    pub fn canonical_bytes(self) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[0..4].copy_from_slice(&self.zone_id.0.to_le_bytes());
        b[4..12].copy_from_slice(&self.local_seq.to_le_bytes());
        b[12..16].copy_from_slice(&(self.epoch as u32).to_le_bytes());
        b
    }
}

// ─── Ord ──────────────────────────────────────────────────────────────────────

impl PartialOrd for LockstepTick {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LockstepTick {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Primary key: zone_id — groups same-zone ticks together in BTreeMaps.
        // Secondary key: local_seq — the actual simulation ordering within a zone.
        // Epoch is excluded from Ord; it is audit-only.
        self.zone_id.cmp(&other.zone_id)
            .then(self.local_seq.cmp(&other.local_seq))
    }
}

impl Default for LockstepTick {
    fn default() -> Self { Self::zero() }
}

impl std::fmt::Display for LockstepTick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tick({}/seq={}/epoch={})", self.zone_id.0, self.local_seq, self.epoch)
    }
}

impl From<u64> for LockstepTick {
    /// Legacy conversion — places tick in global zone.
    fn from(n: u64) -> Self { Self::from_legacy(n) }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_global_seq_zero() {
        let t = LockstepTick::zero();
        assert_eq!(t.zone_id(), ZoneId::GLOBAL);
        assert_eq!(t.local_seq(), 0);
    }

    #[test]
    fn next_increments_local_seq_only() {
        let t = LockstepTick::at(ZoneId::new(2), 41, 7);
        let n = t.next();
        assert_eq!(n.local_seq(), 42);
        assert_eq!(n.zone_id(),   ZoneId::new(2));
        assert_eq!(n.epoch(),     7); // epoch unchanged by tick loop
    }

    #[test]
    fn same_zone_ordering() {
        let a = LockstepTick::at(ZoneId::new(1), 5, 0);
        let b = LockstepTick::at(ZoneId::new(1), 9, 0);
        assert!(a < b);
        assert_eq!(a.causal_cmp(b), CausalOrder::Before);
        assert_eq!(b.causal_cmp(a), CausalOrder::After);
    }

    #[test]
    fn cross_zone_is_concurrent() {
        let a = LockstepTick::at(ZoneId::new(1), 100, 0);
        let b = LockstepTick::at(ZoneId::new(2), 1,   0);
        // a has higher local_seq but different zone — cannot compare causally
        assert_eq!(a.causal_cmp(b), CausalOrder::Concurrent);
        // Ord sorts by zone_id first
        assert!(a < b); // zone 1 < zone 2
    }

    #[test]
    fn btreemap_groups_by_zone() {
        use std::collections::BTreeMap;
        let mut m: BTreeMap<LockstepTick, &str> = BTreeMap::new();
        m.insert(LockstepTick::at(ZoneId::new(2), 0, 0), "z2-t0");
        m.insert(LockstepTick::at(ZoneId::new(1), 0, 0), "z1-t0");
        m.insert(LockstepTick::at(ZoneId::new(1), 1, 0), "z1-t1");
        // Zone 1 entries come before Zone 2
        let keys: Vec<_> = m.keys().collect();
        assert_eq!(keys[0].zone_id(), ZoneId::new(1));
        assert_eq!(keys[1].zone_id(), ZoneId::new(1));
        assert_eq!(keys[2].zone_id(), ZoneId::new(2));
    }

    #[test]
    fn is_lagging_same_zone() {
        let current = LockstepTick::at(ZoneId::GLOBAL, 100, 0);
        assert!(LockstepTick::at(ZoneId::GLOBAL, 90, 0).is_lagging(current, 5));
        assert!(!LockstepTick::at(ZoneId::GLOBAL, 97, 0).is_lagging(current, 5));
    }

    #[test]
    fn canonical_bytes_stable() {
        let t = LockstepTick::at(ZoneId::new(3), 1000, 42);
        let b = t.canonical_bytes();
        // zone_id = 3
        assert_eq!(u32::from_le_bytes(b[0..4].try_into().unwrap()), 3);
        // local_seq = 1000
        assert_eq!(u64::from_le_bytes(b[4..12].try_into().unwrap()), 1000);
    }

    #[test]
    fn from_legacy_global_zone() {
        let t = LockstepTick::from_legacy(77);
        assert_eq!(t.zone_id(),   ZoneId::GLOBAL);
        assert_eq!(t.local_seq(), 77);
    }
}
