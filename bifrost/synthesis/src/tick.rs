//! [`SynthesisTick`] — single tick execution producing a list of intents.
//!
//! Implements the Synthesis faction tick loop:
//!
//! ```text
//! Tick:
//!   1. Sense World  (BIFROST snapshot → WorldModel update)
//!   2. Update faction strategy  (StrategyEngine::evaluate)
//!   3. Emit intents (FactionIntent per active agent)
//!   4. [Caller] Validate via WAC IVL
//!   5. [Caller] Execute via WAC + world engine
//! ```

use std::collections::BTreeMap;

use crate::agent::{AgentRole, AgentState};
use crate::faction::{AiFaction, ZoneId};
use crate::intent::{FactionIntent, IntentPriority, IntentType};
use crate::strategy::{StrategyEngine, StrategyGoal};

// ─── TickInput ───────────────────────────────────────────────────────────────

/// External world state fed into the Synthesis tick.
///
/// In production this is populated from the BIFROST world snapshot.
#[derive(Debug, Default)]
pub struct TickInput {
    pub tick:             u64,
    /// Resource score per zone (0.0–1.0).
    pub zone_resources:   BTreeMap<ZoneId, f32>,
    /// Player fortress presence per zone.
    pub player_fortresses: BTreeMap<ZoneId, f32>,
    /// Player economy fraction (0.0–1.0).
    pub player_economy_fraction: f32,
}

// ─── TickOutput ──────────────────────────────────────────────────────────────

/// The result of a single Synthesis tick.
#[derive(Debug, Default)]
pub struct TickOutput {
    /// Intents to be validated and executed.
    pub intents: Vec<FactionIntent>,
    /// Goals pursued this tick (for logging / debugging).
    pub goals:   Vec<StrategyGoal>,
}

// ─── SynthesisTick ───────────────────────────────────────────────────────────

/// Executes a single Synthesis faction tick.
pub struct SynthesisTick<'a> {
    pub faction: &'a mut AiFaction,
    engine:  StrategyEngine,
}

impl<'a> SynthesisTick<'a> {
    pub fn new(faction: &'a mut AiFaction) -> Self {
        Self { faction, engine: StrategyEngine }
    }

    /// Run the full tick cycle and return a [`TickOutput`].
    pub fn run(&mut self, input: TickInput) -> TickOutput {
        // 1. Sense World — update faction's world model.
        self.faction.world_model.update(input.tick, input.zone_resources);
        self.faction.world_model.player_fortresses = input.player_fortresses;
        self.faction.world_model.player_economy_fraction = input.player_economy_fraction;

        // 2. Update strategy.
        let goals = self.engine.evaluate(&self.faction.world_model);

        // Push highest-priority goal if stack is empty.
        if self.faction.goal_stack.is_empty() {
            if let Some(g) = goals.first() {
                self.faction.push_goal(g.clone());
            }
        }

        // 3. Emit intents — one per executing agent.
        let mut intents = Vec::new();
        for agent in &mut self.faction.agents {
            agent.tick();
            if agent.state != AgentState::Executing { continue; }

            let intent_type = goal_to_intent(
                self.faction.goal_stack.last(),
                agent.zone_id.as_deref(),
                self.faction.world_model.highest_value_zone().map(|s| s.as_str()),
                self.faction.id.as_str(),
            );

            if let Some(it) = intent_type {
                intents.push(FactionIntent::new(
                    &self.faction.id,
                    agent.id,
                    input.tick,
                    priority_from_role(agent.role),
                    it,
                ));
            }
        }

        TickOutput { intents, goals }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn goal_to_intent(
    goal:         Option<&StrategyGoal>,
    agent_zone:   Option<&str>,
    best_zone:    Option<&str>,
    faction_id:   &str,
) -> Option<IntentType> {
    match goal? {
        StrategyGoal::ExpandTerritory => {
            let zone = best_zone.or(agent_zone)?;
            Some(IntentType::CaptureZone { zone_id: zone.to_string() })
        }
        StrategyGoal::AdaptBiome { zone_id } => Some(IntentType::AdaptBiome {
            zone_id:           zone_id.clone(),
            temperature_delta: 0.15,
            humidity_delta:    -0.20,
        }),
        StrategyGoal::DestabiliseEconomy => {
            Some(IntentType::CompileAsset {
                spec:       format!(
                    "hostile dungeon with unstable loot markets and roaming AI traders \
                     in zone controlled by faction {}",
                    faction_id
                ),
                zone_id:    agent_zone.unwrap_or("unknown").to_string(),
                asset_type: "loot_table".into(),
                seed:       42,
            })
        }
        StrategyGoal::DefendZone { zone_id } => Some(IntentType::CompileAsset {
            spec:       format!("defensive fortress village with stone walls in zone {zone_id}"),
            zone_id:    zone_id.clone(),
            asset_type: "tile_map".into(),
            seed:       99,
        }),
        StrategyGoal::AssaultZone { zone_id } => {
            Some(IntentType::CaptureZone { zone_id: zone_id.clone() })
        }
        StrategyGoal::AdvanceTechnology => Some(IntentType::ResearchTech { investment: 10.0 }),
        StrategyGoal::SpyAuction => Some(IntentType::ObserveAuction),
    }
}

fn priority_from_role(role: AgentRole) -> IntentPriority {
    match role {
        AgentRole::Core   => IntentPriority::Critical,
        AgentRole::Region => IntentPriority::High,
        AgentRole::Squad  => IntentPriority::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentNode;
    use crate::faction::AiFaction;

    #[test]
    fn tick_produces_no_intents_without_active_agents() {
        let mut faction = AiFaction::new("synthesis", "Synthesis");
        let mut tick = SynthesisTick::new(&mut faction);
        let out = tick.run(TickInput::default());
        assert!(out.intents.is_empty());
    }

    #[test]
    fn tick_with_executing_agent_produces_intent() {
        let mut faction = AiFaction::new("synthesis", "Synthesis");
        let mut agent = AgentNode::new(AgentRole::Squad);
        agent.assign("zone-A3", "expand");
        faction.agents.push(agent);
        faction.push_goal(StrategyGoal::SpyAuction);

        let mut tick = SynthesisTick::new(&mut faction);
        let out = tick.run(TickInput::default());
        assert_eq!(out.intents.len(), 1);
        assert!(matches!(out.intents[0].intent, IntentType::ObserveAuction));
    }
}
