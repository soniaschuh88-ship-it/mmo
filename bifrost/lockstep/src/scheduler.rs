//! LockstepScheduler — zone-aware deterministic tick orchestrator.
//!
//! Each scheduler owns one `ShardId`. Submissions for a different zone are
//! rejected — use a separate scheduler per zone.
//!
//! # Advance Rule
//!
//! ```text
//! tick N → N+1 requires: ∀ peer ∈ registered: peer.last_ack >= N
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use bifrost_chunk::PeerId;
use bifrost_vis::VoxelProgram;

use crate::barrier::TickBarrier;
use crate::budget::{BudgetError, TickBudget, TickUsage};
use crate::input::InputBuffer;
use crate::tick::{LockstepTick, ShardId};

const MAX_AHEAD_TICKS: u64 = 8;

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("tick zone {tick_zone} does not match scheduler zone {sched_zone}")]
    ZoneMismatch { tick_zone: u32, sched_zone: u32 },
    #[error("submission for seq {seq} is too far ahead (current: {current})")]
    TooFarAhead { seq: u64, current: u64 },
    #[error("submission for seq {0} is in the past")]
    TickInPast(u64),
    #[error("tick budget exceeded: {0}")]
    BudgetExceeded(#[from] BudgetError),
}

/// Produced when the world tick can advance.
#[derive(Debug)]
pub struct TickAdvance {
    pub completed_tick: LockstepTick,
    /// Merged inputs for the completed tick, sorted deterministically by PeerId.
    pub inputs: BTreeMap<PeerId, VoxelProgram>,
}

/// Orchestrates deterministic lockstep execution for a single zone.
#[derive(Debug, Serialize, Deserialize)]
pub struct LockstepScheduler {
    zone_id:          ShardId,
    current_tick:     LockstepTick,
    tick_duration_ms: u16,
    barrier:          TickBarrier,
    inputs:           InputBuffer,
    /// Per-tick budget configuration.
    pub budget:       TickBudget,
    /// Accumulated usage for the current tick. Reset on every advance.
    usage:            TickUsage,
}

impl LockstepScheduler {
    /// Create a scheduler for `ShardId::GLOBAL` (backward-compatible default).
    pub fn new(tick_duration_ms: u16) -> Self {
        Self::for_zone(ShardId::GLOBAL, tick_duration_ms)
    }

    /// Create a scheduler for a specific zone.
    pub fn for_zone(zone_id: ShardId, tick_duration_ms: u16) -> Self {
        Self {
            zone_id,
            current_tick: LockstepTick::zone_start(zone_id, 0),
            tick_duration_ms,
            barrier: TickBarrier::new(),
            inputs:  InputBuffer::new(),
            budget:  TickBudget::default(),
            usage:   TickUsage::default(),
        }
    }

    /// Replace the budget configuration (e.g. production vs. dev limits).
    pub fn with_budget(mut self, budget: TickBudget) -> Self {
        self.budget = budget;
        self
    }

    pub fn register_peer(&mut self, peer: PeerId) { self.barrier.register(peer); }
    pub fn evict_peer(&mut self, peer: &PeerId)   { self.barrier.evict(peer); }

    /// Submit a peer's `VoxelProgram` for `tick`.
    ///
    /// Rejects ticks from the wrong zone, past ticks, and submissions too far
    /// ahead of the current tick.
    pub fn submit_input(
        &mut self,
        peer:    PeerId,
        tick:    LockstepTick,
        program: VoxelProgram,
    ) -> Result<(), SchedulerError> {
        if tick.zone_id() != self.zone_id {
            return Err(SchedulerError::ZoneMismatch {
                tick_zone:  tick.zone_id().0,
                sched_zone: self.zone_id.0,
            });
        }
        if tick < self.current_tick {
            return Err(SchedulerError::TickInPast(tick.local_seq()));
        }
        if tick.local_seq() > self.current_tick.local_seq() + MAX_AHEAD_TICKS {
            return Err(SchedulerError::TooFarAhead {
                seq:     tick.local_seq(),
                current: self.current_tick.local_seq(),
            });
        }
        // Budget check — updates usage on accept, rejects with BudgetError otherwise
        self.usage = self.budget.check_program(&program, &self.usage)?;
        self.inputs.submit(tick, peer, program);
        Ok(())
    }

    pub fn record_ack(&mut self, peer: PeerId, tick: LockstepTick) {
        self.barrier.ack(peer, tick);
    }

    /// Attempt to advance the world tick.
    ///
    /// Returns `Some(TickAdvance)` if the barrier clears, `None` if peers are
    /// still lagging.
    pub fn try_advance(&mut self) -> Option<TickAdvance> {
        if !self.barrier.can_advance(self.current_tick) {
            return None;
        }
        let completed = self.current_tick;
        let inputs = self.inputs.get_tick_inputs(completed).cloned().unwrap_or_default();
        self.current_tick = self.current_tick.next();
        self.inputs.evict_before(self.current_tick);
        // Reset usage counters for the new tick
        self.usage = TickUsage::default();
        Some(TickAdvance { completed_tick: completed, inputs })
    }

    pub fn current_tick(&self)     -> LockstepTick  { self.current_tick }
    pub fn tick_duration_ms(&self) -> u16           { self.tick_duration_ms }
    pub fn zone_id(&self)          -> ShardId        { self.zone_id }
    pub fn peer_count(&self)       -> usize         { self.barrier.peer_count() }
    pub fn current_usage(&self)    -> &TickUsage    { &self.usage }
    pub fn budget(&self)           -> &TickBudget   { &self.budget }

    pub fn lagging_peers(&self) -> Vec<PeerId> {
        self.barrier.lagging_peers(self.current_tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bifrost_vis::{InstructionPayload, SetVoxelPayload, VoxelCoord, VoxelProgram};

    fn p(seed: u8) -> PeerId { PeerId([seed; 32]) }
    fn t(seq: u64) -> LockstepTick { LockstepTick::from_legacy(seq) }

    fn empty_prog() -> VoxelProgram { VoxelProgram::new() }

    fn prog_set(n: i32) -> VoxelProgram {
        let mut p = VoxelProgram::new();
        for x in 0..n {
            p.push(1, InstructionPayload::SetVoxel(SetVoxelPayload {
                position: VoxelCoord::new(x, 0, 0), material: 1,
            })).unwrap();
        }
        p
    }

    #[test]
    fn no_advance_without_ack() {
        let mut s = LockstepScheduler::new(50);
        s.register_peer(p(1));
        assert!(s.try_advance().is_none());
    }

    #[test]
    fn advance_when_all_acked() {
        let mut s = LockstepScheduler::new(50);
        s.register_peer(p(1)); s.register_peer(p(2));
        s.record_ack(p(1), t(0));
        assert!(s.try_advance().is_none());
        s.record_ack(p(2), t(0));
        let adv = s.try_advance().expect("should advance");
        assert_eq!(adv.completed_tick.local_seq(), 0);
        assert_eq!(s.current_tick().local_seq(), 1);
    }

    #[test]
    fn inputs_in_advance() {
        let mut s = LockstepScheduler::new(50);
        s.register_peer(p(1));
        s.submit_input(p(1), t(0), prog_set(5)).unwrap();
        s.record_ack(p(1), t(0));
        let adv = s.try_advance().unwrap();
        assert!(adv.inputs.contains_key(&p(1)));
    }

    #[test]
    fn zone_mismatch_rejected() {
        let mut s = LockstepScheduler::for_zone(ShardId::new(1), 50);
        s.register_peer(p(1));
        // Submission for zone 2 → rejected
        let wrong_zone = LockstepTick::at(ShardId::new(2), 0, 0);
        assert!(matches!(
            s.submit_input(p(1), wrong_zone, empty_prog()),
            Err(SchedulerError::ZoneMismatch { .. })
        ));
    }

    #[test]
    fn too_far_ahead_rejected() {
        let mut s = LockstepScheduler::new(50);
        s.register_peer(p(1));
        assert!(s.submit_input(p(1), t(100), empty_prog()).is_err());
    }

    #[test]
    fn past_tick_rejected() {
        let mut s = LockstepScheduler::new(50);
        s.register_peer(p(1));
        s.record_ack(p(1), t(0));
        s.try_advance();
        assert!(s.submit_input(p(1), t(0), empty_prog()).is_err());
    }

    #[test]
    fn evict_unblocks() {
        let mut s = LockstepScheduler::new(50);
        s.register_peer(p(1)); s.register_peer(p(2));
        s.record_ack(p(1), t(0));
        s.evict_peer(&p(2));
        assert!(s.try_advance().is_some());
    }

    #[test]
    fn sequential_advances() {
        let mut s = LockstepScheduler::new(50);
        s.register_peer(p(1));
        for expected in 0u64..5 {
            s.record_ack(p(1), t(expected));
            let adv = s.try_advance().unwrap();
            assert_eq!(adv.completed_tick.local_seq(), expected);
        }
        assert_eq!(s.current_tick().local_seq(), 5);
    }

    #[test]
    fn for_zone_starts_in_correct_zone() {
        let s = LockstepScheduler::for_zone(ShardId::new(7), 50);
        assert_eq!(s.current_tick().zone_id(), ShardId::new(7));
        assert_eq!(s.current_tick().local_seq(), 0);
    }

    #[test]
    fn budget_enforced_on_submit() {
        // Tight budget: max 2 total instructions
        let mut s = LockstepScheduler::new(50)
            .with_budget(TickBudget { max_total: 2, max_physics: 100, max_ai: 100, max_programs: 10 });
        s.register_peer(p(1));
        // 2 setvoxels: accepted
        assert!(s.submit_input(p(1), t(0), prog_set(2)).is_ok());
        // 1 more: rejected — total would be 3 > 2
        assert!(matches!(
            s.submit_input(p(1), t(0), prog_set(1)),
            Err(SchedulerError::BudgetExceeded(_))
        ));
    }

    #[test]
    fn budget_resets_after_advance() {
        let mut s = LockstepScheduler::new(50)
            .with_budget(TickBudget { max_total: 2, max_physics: 100, max_ai: 100, max_programs: 10 });
        s.register_peer(p(1));
        // Fill budget for tick 0
        s.submit_input(p(1), t(0), prog_set(2)).unwrap();
        // Advance to tick 1
        s.record_ack(p(1), t(0));
        s.try_advance().unwrap();
        // Budget reset — can submit again for tick 1
        assert!(s.submit_input(p(1), t(1), prog_set(2)).is_ok());
    }

    #[test]
    fn budget_usage_reported() {
        let mut s = LockstepScheduler::new(50);
        s.register_peer(p(1));
        assert_eq!(s.current_usage().total, 0);
        s.submit_input(p(1), t(0), prog_set(3)).unwrap();
        assert_eq!(s.current_usage().total, 3);
    }
}
