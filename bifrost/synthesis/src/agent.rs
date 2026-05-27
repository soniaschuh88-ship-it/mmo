//! [`AgentNode`] — squad/clan-level agent within the Synthesis faction.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::faction::ZoneId;

// ─── AgentNode ────────────────────────────────────────────────────────────────

/// A single Synthesis agent node.
///
/// Scale mapping:
/// - `Squad` → 1 AgentNode per player-equivalent squad
/// - `Region` → 1 SubAi controlling a full zone
/// - `Core` → 1 global strategist (backed by NVIDIA NIM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    pub id:         Uuid,
    pub role:       AgentRole,
    pub state:      AgentState,

    /// Zone this agent is currently operating in.
    pub zone_id:    Option<ZoneId>,

    /// Current objective this agent is pursuing.
    pub objective:  Option<String>,

    /// Trust score (higher = more authority, more WAC compile budget).
    pub trust_score: f32,

    /// Number of ticks this agent has been active.
    pub ticks_active: u64,
}

impl AgentNode {
    pub fn new(role: AgentRole) -> Self {
        Self {
            id:           Uuid::new_v4(),
            role,
            state:        AgentState::Idle,
            zone_id:      None,
            objective:    None,
            trust_score:  1.0,
            ticks_active: 0,
        }
    }

    /// Advance the agent by one tick.
    pub fn tick(&mut self) {
        self.ticks_active += 1;
    }

    /// Assign the agent to a zone with an objective.
    pub fn assign(&mut self, zone_id: impl Into<ZoneId>, objective: impl Into<String>) {
        self.zone_id   = Some(zone_id.into());
        self.objective = Some(objective.into());
        self.state     = AgentState::Executing;
    }

    /// Mark the agent as idle (objective complete or cancelled).
    pub fn release(&mut self) {
        self.objective = None;
        self.state     = AgentState::Idle;
    }
}

// ─── AgentRole ───────────────────────────────────────────────────────────────

/// The role / scale tier of a Synthesis agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Squad-level tactical agent (1 per player equivalent).
    Squad,
    /// Region controller (1 per zone).
    Region,
    /// Global strategist — single instance, backed by NVIDIA NIM.
    Core,
}

// ─── AgentState ──────────────────────────────────────────────────────────────

/// Lifecycle state of an [`AgentNode`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    /// Waiting for assignment.
    Idle,
    /// Actively executing an objective.
    Executing,
    /// Temporarily withdrawn (e.g. trust score too low).
    Suspended,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_starts_idle() {
        let a = AgentNode::new(AgentRole::Squad);
        assert_eq!(a.state, AgentState::Idle);
        assert!(a.objective.is_none());
    }

    #[test]
    fn agent_assign_transitions_state() {
        let mut a = AgentNode::new(AgentRole::Region);
        a.assign("zone-A3", "destabilize economy");
        assert_eq!(a.state, AgentState::Executing);
        assert_eq!(a.zone_id.as_deref(), Some("zone-A3"));
    }

    #[test]
    fn agent_tick_increments() {
        let mut a = AgentNode::new(AgentRole::Core);
        a.tick();
        a.tick();
        assert_eq!(a.ticks_active, 2);
    }
}
