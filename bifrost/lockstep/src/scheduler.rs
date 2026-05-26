//! LockstepScheduler — the main lockstep tick orchestrator.
//!
//! The scheduler:
//! 1. Accepts `VoxelProgram` submissions from peers for upcoming ticks
//! 2. Records tick acknowledgments
//! 3. Advances the world tick when all peers have acked the current tick
//! 4. Returns a `TickAdvance` with the merged inputs for execution

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use bifrost_chunk::PeerId;
use bifrost_vis::VoxelProgram;

use crate::barrier::TickBarrier;
use crate::input::InputBuffer;
use crate::tick::LockstepTick;

/// Maximum number of ticks a peer can be ahead of the current tick
/// before submissions are rejected (prevents unbounded buffer growth).
const MAX_AHEAD_TICKS: u64 = 8;

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("peer {0:?} is not registered")]
    UnregisteredPeer(PeerId),
    #[error("submission for tick {tick} is too far ahead (current: {current})")]
    TooFarAhead { tick: u64, current: u64 },
    #[error("submission for tick {0} is in the past")]
    TickInPast(u64),
}

/// Produced when the world tick can advance.
#[derive(Debug)]
pub struct TickAdvance {
    /// The tick that just completed.
    pub completed_tick: LockstepTick,
    /// The merged `VoxelProgram` inputs for that tick, sorted by peer.
    /// Deterministic: BTreeMap iteration order is by PeerId (byte order).
    pub inputs: BTreeMap<PeerId, VoxelProgram>,
}

/// Orchestrates deterministic lockstep execution across a peer swarm.
///
/// # Advance Rule
///
/// ```text
/// tick N → N+1 requires: ∀ peer ∈ registered: peer.last_ack >= N
/// ```
///
/// Slow peers apply backpressure. Use `evict_peer` to remove unresponsive ones.
#[derive(Debug, Serialize, Deserialize)]
pub struct LockstepScheduler {
    current_tick:    LockstepTick,
    tick_duration_ms: u16,
    barrier:         TickBarrier,
    inputs:          InputBuffer,
}

impl LockstepScheduler {
    /// Create a new scheduler.
    ///
    /// `tick_duration_ms` is the target duration for each tick. The scheduler
    /// itself does not enforce this timing — callers are responsible for
    /// driving the tick loop at the right cadence.
    pub fn new(tick_duration_ms: u16) -> Self {
        Self {
            current_tick: LockstepTick::zero(),
            tick_duration_ms,
            barrier: TickBarrier::new(),
            inputs: InputBuffer::new(),
        }
    }

    /// Add a peer to the simulation.
    pub fn register_peer(&mut self, peer: PeerId) {
        self.barrier.register(peer);
    }

    /// Remove a peer (e.g. on disconnect).
    pub fn evict_peer(&mut self, peer: &PeerId) {
        self.barrier.evict(peer);
    }

    /// Submit a peer's `VoxelProgram` for `tick`.
    pub fn submit_input(
        &mut self,
        peer: PeerId,
        tick: LockstepTick,
        program: VoxelProgram,
    ) -> Result<(), SchedulerError> {
        // Reject past ticks
        if tick < self.current_tick {
            return Err(SchedulerError::TickInPast(tick.0));
        }
        // Reject submissions too far in the future
        if tick.0 > self.current_tick.0 + MAX_AHEAD_TICKS {
            return Err(SchedulerError::TooFarAhead {
                tick:    tick.0,
                current: self.current_tick.0,
            });
        }
        self.inputs.submit(tick, peer, program);
        Ok(())
    }

    /// Record that `peer` has finished simulating `tick`.
    pub fn record_ack(&mut self, peer: PeerId, tick: LockstepTick) {
        self.barrier.ack(peer, tick);
    }

    /// Attempt to advance the world tick.
    ///
    /// Returns `Some(TickAdvance)` if the barrier clears and the tick advances,
    /// `None` if peers are still lagging.
    ///
    /// After a successful advance, the old tick's input buffer is evicted.
    pub fn try_advance(&mut self) -> Option<TickAdvance> {
        if !self.barrier.can_advance(self.current_tick) {
            return None;
        }

        let completed = self.current_tick;
        let inputs = self.inputs
            .get_tick_inputs(completed)
            .cloned()
            .unwrap_or_default();

        // Advance
        self.current_tick = self.current_tick.next();
        // Evict inputs for completed tick
        self.inputs.evict_before(self.current_tick);

        Some(TickAdvance { completed_tick: completed, inputs })
    }

    /// Current world tick (the tick being built, not yet executed).
    pub fn current_tick(&self) -> LockstepTick {
        self.current_tick
    }

    /// Target duration of a single tick in milliseconds.
    pub fn tick_duration_ms(&self) -> u16 {
        self.tick_duration_ms
    }

    /// Peers that have not yet acked the current tick.
    pub fn lagging_peers(&self) -> Vec<PeerId> {
        self.barrier.lagging_peers(self.current_tick)
    }

    /// Number of registered peers.
    pub fn peer_count(&self) -> usize {
        self.barrier.peer_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bifrost_vis::{InstructionPayload, SetVoxelPayload, VoxelCoord, VoxelProgram};

    fn test_peer(seed: u8) -> PeerId {
        PeerId([seed; 32])
    }

    fn empty_program() -> VoxelProgram {
        VoxelProgram::new()
    }

    fn program_with_set(x: i32) -> VoxelProgram {
        let mut p = VoxelProgram::new();
        p.push(1, InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::new(x, 0, 0),
            material: 1,
        })).unwrap();
        p
    }

    #[test]
    fn no_advance_without_ack() {
        let mut sched = LockstepScheduler::new(50);
        sched.register_peer(test_peer(1));
        assert!(sched.try_advance().is_none());
    }

    #[test]
    fn advance_when_all_acked() {
        let mut sched = LockstepScheduler::new(50);
        let p1 = test_peer(1);
        let p2 = test_peer(2);
        sched.register_peer(p1);
        sched.register_peer(p2);

        sched.record_ack(p1, LockstepTick(0));
        assert!(sched.try_advance().is_none()); // p2 not acked

        sched.record_ack(p2, LockstepTick(0));
        let adv = sched.try_advance().expect("should advance");
        assert_eq!(adv.completed_tick, LockstepTick(0));
        assert_eq!(sched.current_tick(), LockstepTick(1));
    }

    #[test]
    fn inputs_included_in_advance() {
        let mut sched = LockstepScheduler::new(50);
        let p1 = test_peer(1);
        sched.register_peer(p1);

        sched.submit_input(p1, LockstepTick(0), program_with_set(5)).unwrap();
        sched.record_ack(p1, LockstepTick(0));

        let adv = sched.try_advance().unwrap();
        assert!(adv.inputs.contains_key(&p1));
    }

    #[test]
    fn too_far_ahead_rejected() {
        let mut sched = LockstepScheduler::new(50);
        let p = test_peer(1);
        sched.register_peer(p);
        assert!(sched.submit_input(p, LockstepTick(100), empty_program()).is_err());
    }

    #[test]
    fn past_tick_rejected() {
        let mut sched = LockstepScheduler::new(50);
        let p = test_peer(1);
        sched.register_peer(p);
        sched.record_ack(p, LockstepTick(0));
        sched.try_advance(); // advance to tick 1

        assert!(sched.submit_input(p, LockstepTick(0), empty_program()).is_err());
    }

    #[test]
    fn evict_peer_unblocks() {
        let mut sched = LockstepScheduler::new(50);
        let p1 = test_peer(1);
        let p2 = test_peer(2);
        sched.register_peer(p1);
        sched.register_peer(p2);
        sched.record_ack(p1, LockstepTick(0));

        sched.evict_peer(&p2);
        assert!(sched.try_advance().is_some());
    }

    #[test]
    fn sequential_advances() {
        let mut sched = LockstepScheduler::new(50);
        let p = test_peer(1);
        sched.register_peer(p);

        for expected_tick in 0u64..5 {
            sched.record_ack(p, LockstepTick(expected_tick));
            let adv = sched.try_advance().unwrap();
            assert_eq!(adv.completed_tick.0, expected_tick);
        }
        assert_eq!(sched.current_tick(), LockstepTick(5));
    }
}
