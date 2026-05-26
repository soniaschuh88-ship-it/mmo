//! # bifrost-lockstep — Lockstep Tick Scheduler
//!
//! Enforces the core synchrony invariant of the Bifrost Layer:
//!
//! ```text
//! Tick N+1 starts ONLY when ∀ peer ∈ registered_peers: peer.last_ack >= N
//! ```
//!
//! More players never means more lag — late peers apply backpressure, not
//! server load. Unresponsive peers are evicted, unblocking the swarm.
//!
//! ## Key types
//!
//! - [`ZoneId`] — spatial simulation partition identifier
//! - [`LockstepTick`] — zone-local tick (zone_id + local_seq + epoch)
//! - [`CausalOrder`] — cross-zone causal comparison result
//! - [`TickBarrier`] — tracks per-peer acknowledgments
//! - [`InputBuffer`] — stores per-peer `VoxelProgram`s per tick
//! - [`LockstepScheduler`] — drives tick advancement + input aggregation

pub mod barrier;
pub mod budget;
pub mod input;
pub mod scheduler;
pub mod tick;

pub use barrier::TickBarrier;
pub use budget::{BudgetError, EventClass, TickBudget, TickUsage, TickUsageHeadroom};
pub use input::InputBuffer;
pub use scheduler::{LockstepScheduler, SchedulerError, TickAdvance};
pub use tick::{CausalOrder, LockstepTick, ZoneId};
