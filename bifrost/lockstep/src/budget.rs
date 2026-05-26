//! TickBudget — per-tick event budget with priority scheduling.
//!
//! # Motivation
//!
//! Unbounded event submission per tick creates two failure modes:
//! 1. Feedback loops: system A emits event → system B emits 10 events → ...
//! 2. Starvation: one zone or peer floods the tick with low-priority updates.
//!
//! The budget enforces hard caps per event class, with priority ordering:
//!
//! ```text
//! Physics > Structural > Visual > AI (future)
//! ```
//!
//! Physics gets the most headroom because it is the only class that MUST be
//! consistent across all peers. AI events are strictly rate-limited because
//! LLM-generated events are expensive and non-deterministic by nature.
//!
//! # Priority scheduling semantics
//!
//! Higher-priority classes consume budget first. If the total budget is
//! nearly exhausted, only Physics and Structural submissions are accepted.
//! Visual submissions are the first to be shed under pressure.
//!
//! See [`TickBudget::check_program`] for the full rejection logic.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use bifrost_vis::{VoxelOpcode, VoxelProgram};

// ─── EventClass ───────────────────────────────────────────────────────────────

/// The priority class of a VIS instruction.
///
/// Used for budget allocation and priority scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventClass {
    /// Deterministic world-state mutations (DamageField, SimExplosion, etc.).
    /// Must be consistent across all peers. Highest priority.
    Physics,
    /// Direct voxel placement or removal (SetVoxel, FillBox).
    /// Second priority.
    Structural,
    /// Visual-only transformations (MarchMaterial).
    /// Shed first under budget pressure.
    Visual,
    /// AI/NPC-generated events — reserved for Phase 2.
    /// Strictly rate-limited.
    Ai,
}

impl EventClass {
    /// Classify a `VoxelOpcode` by event class.
    pub fn of_opcode(op: VoxelOpcode) -> Self {
        match op {
            VoxelOpcode::DamageField
            | VoxelOpcode::SimExplosion
            | VoxelOpcode::SimWater
            | VoxelOpcode::SimFire
            | VoxelOpcode::SimDebris
            | VoxelOpcode::SphereCut => Self::Physics,

            VoxelOpcode::FillBox
            | VoxelOpcode::SetVoxel => Self::Structural,

            VoxelOpcode::MarchMaterial => Self::Visual,
        }
    }

    /// Priority rank — higher is more important. Used for budget shed ordering.
    pub fn priority(self) -> u8 {
        match self {
            Self::Physics    => 3,
            Self::Structural => 2,
            Self::Visual     => 1,
            Self::Ai         => 0,
        }
    }
}

// ─── TickBudget ───────────────────────────────────────────────────────────────

/// Per-tick event budget configuration.
///
/// All limits are instruction counts (not byte counts). The scheduler
/// enforces these limits before accepting a `VoxelProgram` submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickBudget {
    /// Hard cap: total instructions across ALL programs in one tick.
    pub max_total:     u32,
    /// Soft cap: physics-class instructions (DamageField, SimExplosion, …).
    /// Typically the largest allocation since physics must be consistent.
    pub max_physics:   u32,
    /// Hard cap: AI-generated instructions (NPC behaviors — Phase 2).
    /// Kept low to prevent LLM feedback loops.
    pub max_ai:        u32,
    /// Hard cap: number of distinct VoxelPrograms per tick.
    /// Limits one-program-per-peer at scale.
    pub max_programs:  u32,
}

impl TickBudget {
    /// Production defaults.
    pub fn default_production() -> Self {
        Self {
            max_total:    2_000,
            max_physics:  1_500,
            max_ai:       50,
            max_programs: 200,
        }
    }

    /// Relaxed limits for local development and testing.
    pub fn default_dev() -> Self {
        Self {
            max_total:    10_000,
            max_physics:  8_000,
            max_ai:       500,
            max_programs: 1_000,
        }
    }

    /// No limits — for benchmarks and unit tests that verify
    /// correctness regardless of scale.
    pub fn unlimited() -> Self {
        Self {
            max_total:    u32::MAX,
            max_physics:  u32::MAX,
            max_ai:       u32::MAX,
            max_programs: u32::MAX,
        }
    }
}

impl Default for TickBudget {
    fn default() -> Self {
        Self::default_dev()
    }
}

// ─── TickUsage ────────────────────────────────────────────────────────────────

/// Accumulated usage counters for the current tick.
///
/// Reset to `default()` on every successful `try_advance`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TickUsage {
    pub total:     u32,
    pub physics:   u32,
    pub ai:        u32,
    pub programs:  u32,
}

impl TickUsage {
    /// Add `program`'s contribution to the usage counters.
    pub fn account_for(&self, program: &VoxelProgram) -> Self {
        let mut physics = self.physics;
        let mut ai      = self.ai;

        for instr in &program.instructions {
            match EventClass::of_opcode(instr.opcode) {
                EventClass::Physics    => physics += 1,
                EventClass::Ai         => ai      += 1,
                EventClass::Structural
                | EventClass::Visual   => {}
            }
        }

        let count = program.instructions.len() as u32;
        Self {
            total:    self.total    + count,
            physics,
            ai,
            programs: self.programs + 1,
        }
    }

    /// Headroom remaining under `budget`.
    pub fn remaining(&self, budget: &TickBudget) -> TickUsageHeadroom {
        TickUsageHeadroom {
            total:    budget.max_total.saturating_sub(self.total),
            physics:  budget.max_physics.saturating_sub(self.physics),
            ai:       budget.max_ai.saturating_sub(self.ai),
            programs: budget.max_programs.saturating_sub(self.programs),
        }
    }
}

/// Remaining capacity in each budget dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickUsageHeadroom {
    pub total:    u32,
    pub physics:  u32,
    pub ai:       u32,
    pub programs: u32,
}

// ─── BudgetError ──────────────────────────────────────────────────────────────

/// Returned when a program submission would exceed the tick budget.
///
/// Higher-priority classes (Physics, Structural) are never shed to make
/// room for lower-priority classes. The caller must either drop the
/// submission or carry it forward to the next tick.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum BudgetError {
    #[error("program count budget exceeded: {used}/{limit}")]
    ProgramsExceeded { used: u32, limit: u32 },

    #[error("total instruction budget exceeded: {used}/{limit} (program adds {delta})")]
    TotalExceeded { used: u32, limit: u32, delta: u32 },

    #[error("physics instruction budget exceeded: {used}/{limit} (program adds {delta})")]
    PhysicsExceeded { used: u32, limit: u32, delta: u32 },

    #[error("AI instruction budget exceeded: {used}/{limit} (program adds {delta})")]
    AiExceeded { used: u32, limit: u32, delta: u32 },
}

impl BudgetError {
    /// True if this is a high-priority class being shed (should be retried).
    pub fn is_high_priority_shed(&self) -> bool {
        matches!(self, Self::TotalExceeded { .. } | Self::PhysicsExceeded { .. })
    }
}

// ─── Budget check ─────────────────────────────────────────────────────────────

impl TickBudget {
    /// Check whether adding `program` to `current` usage would stay within budget.
    ///
    /// Returns the updated `TickUsage` if accepted, or `BudgetError` if any
    /// limit would be exceeded.
    ///
    /// # Rejection order (priority scheduling)
    ///
    /// 1. Programs cap — prevents per-peer DOS
    /// 2. AI cap — prevents LLM feedback loops (strictest)
    /// 3. Physics cap — prevents explosion cascades
    /// 4. Total cap — final hard stop
    pub fn check_program(
        &self,
        program: &VoxelProgram,
        current: &TickUsage,
    ) -> Result<TickUsage, BudgetError> {
        // 1. Programs cap
        if current.programs >= self.max_programs {
            return Err(BudgetError::ProgramsExceeded {
                used:  current.programs,
                limit: self.max_programs,
            });
        }

        let projected = current.account_for(program);
        let delta_total   = projected.total   - current.total;
        let delta_physics = projected.physics - current.physics;
        let delta_ai      = projected.ai      - current.ai;

        // 2. AI cap (strictest — LLM calls are expensive + nondeterministic)
        if projected.ai > self.max_ai {
            return Err(BudgetError::AiExceeded {
                used:  current.ai,
                limit: self.max_ai,
                delta: delta_ai,
            });
        }

        // 3. Physics cap
        if projected.physics > self.max_physics {
            return Err(BudgetError::PhysicsExceeded {
                used:  current.physics,
                limit: self.max_physics,
                delta: delta_physics,
            });
        }

        // 4. Total cap
        if projected.total > self.max_total {
            return Err(BudgetError::TotalExceeded {
                used:  current.total,
                limit: self.max_total,
                delta: delta_total,
            });
        }

        Ok(projected)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use bifrost_vis::{
        DamageFieldPayload, FillBoxPayload, InstructionPayload,
        SetVoxelPayload, SimExplosionPayload, VoxelCoord, VoxelProgram,
    };

    fn prog_with_explosion() -> VoxelProgram {
        let mut p = VoxelProgram::new();
        p.push(0, InstructionPayload::SimExplosion(SimExplosionPayload {
            center: VoxelCoord::new(0, 0, 0), radius: 5, force: 100, result_material: 0,
        })).unwrap();
        p
    }

    fn prog_with_setvoxels(n: usize) -> VoxelProgram {
        let mut p = VoxelProgram::new();
        for i in 0..n {
            p.push(0, InstructionPayload::SetVoxel(SetVoxelPayload {
                position: VoxelCoord::new(i as i32, 0, 0), material: 1,
            })).unwrap();
        }
        p
    }

    fn prog_with_damage(n: usize) -> VoxelProgram {
        let mut p = VoxelProgram::new();
        for i in 0..n {
            p.push(0, InstructionPayload::DamageField(DamageFieldPayload {
                center: VoxelCoord::new(i as i32, 0, 0), radius: 1, damage: 10,
            })).unwrap();
        }
        p
    }

    #[test]
    fn empty_program_always_accepted() {
        let b = TickBudget::default_production();
        let u = TickUsage::default();
        let prog = VoxelProgram::new();
        assert!(b.check_program(&prog, &u).is_ok());
    }

    #[test]
    fn physics_opcode_classified_correctly() {
        assert_eq!(EventClass::of_opcode(VoxelOpcode::SimExplosion), EventClass::Physics);
        assert_eq!(EventClass::of_opcode(VoxelOpcode::DamageField),  EventClass::Physics);
        assert_eq!(EventClass::of_opcode(VoxelOpcode::SetVoxel),     EventClass::Structural);
        assert_eq!(EventClass::of_opcode(VoxelOpcode::MarchMaterial),EventClass::Visual);
    }

    #[test]
    fn usage_accounts_physics_correctly() {
        let prog = prog_with_explosion();
        let usage = TickUsage::default().account_for(&prog);
        assert_eq!(usage.total,   1);
        assert_eq!(usage.physics, 1);
        assert_eq!(usage.programs, 1);
    }

    #[test]
    fn usage_accounts_structural_correctly() {
        let prog = prog_with_setvoxels(5);
        let usage = TickUsage::default().account_for(&prog);
        assert_eq!(usage.total,   5);
        assert_eq!(usage.physics, 0); // SetVoxel is Structural, not Physics
        assert_eq!(usage.programs, 1);
    }

    #[test]
    fn total_budget_exceeded() {
        let b = TickBudget { max_total: 3, max_physics: 100, max_ai: 100, max_programs: 100 };
        let u = TickUsage::default();
        let prog = prog_with_setvoxels(4);
        assert!(matches!(b.check_program(&prog, &u), Err(BudgetError::TotalExceeded { .. })));
    }

    #[test]
    fn physics_budget_exceeded() {
        let b = TickBudget { max_total: 10_000, max_physics: 2, max_ai: 100, max_programs: 100 };
        let u = TickUsage::default();
        let prog = prog_with_damage(3);
        assert!(matches!(b.check_program(&prog, &u), Err(BudgetError::PhysicsExceeded { .. })));
    }

    #[test]
    fn programs_cap_exceeded() {
        let b = TickBudget { max_total: 10_000, max_physics: 10_000, max_ai: 100, max_programs: 1 };
        let mut u = TickUsage::default();
        // First program: accepted
        u = b.check_program(&VoxelProgram::new(), &u).unwrap();
        // Second program: rejected — programs cap
        assert!(matches!(
            b.check_program(&VoxelProgram::new(), &u),
            Err(BudgetError::ProgramsExceeded { .. })
        ));
    }

    #[test]
    fn cumulative_budget_tracking() {
        let b = TickBudget { max_total: 10, max_physics: 10_000, max_ai: 100, max_programs: 10 };
        let u = TickUsage::default();
        // 6 setvoxels: accepted
        let u = b.check_program(&prog_with_setvoxels(6), &u).unwrap();
        assert_eq!(u.total, 6);
        // 5 more setvoxels: rejected (would reach 11 > 10)
        assert!(b.check_program(&prog_with_setvoxels(5), &u).is_err());
        // 4 more: accepted (6+4=10, within limit)
        assert!(b.check_program(&prog_with_setvoxels(4), &u).is_ok());
    }

    #[test]
    fn priority_ordering() {
        // Physics has higher priority rank than Structural > Visual > Ai
        assert!(EventClass::Physics.priority() > EventClass::Structural.priority());
        assert!(EventClass::Structural.priority() > EventClass::Visual.priority());
        assert!(EventClass::Visual.priority() > EventClass::Ai.priority());
    }

    #[test]
    fn headroom_calculation() {
        let b = TickBudget { max_total: 100, max_physics: 50, max_ai: 10, max_programs: 20 };
        let u = TickUsage { total: 60, physics: 30, ai: 5, programs: 8 };
        let h = u.remaining(&b);
        assert_eq!(h.total,    40);
        assert_eq!(h.physics,  20);
        assert_eq!(h.ai,       5);
        assert_eq!(h.programs, 12);
    }
}
