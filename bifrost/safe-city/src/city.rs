//! [`SafeCity`] — the central anti-chaos anchor zone.

use serde::{Deserialize, Serialize};

use crate::auction::AuctionHouse;
use crate::zone::ZoneId;

// ─── SafeCity ────────────────────────────────────────────────────────────────

/// The Safe City: the stable economic and social hub of the game world.
///
/// Persists across world runs.  All player and AI economic activity is
/// routed through here via the [`AuctionHouse`].
///
/// Properties:
/// - No combat events
/// - No territory capture
/// - No biome destruction
/// - Only: trade, crafting, skill progression, AI/player interaction
/// - Respawn anchor for cloned players
/// - Auction House — sole global market
#[derive(Debug)]
pub struct SafeCity {
    pub zone_id:          ZoneId,
    pub protection_level: f32,       // 0.0–1.0; 1.0 = fully inviolable

    /// Actions allowed within the city zone.
    pub allowed_actions:  Vec<AllowedAction>,

    /// The city's auction house.
    pub market:           AuctionHouse,

    /// Crafting validation rules.
    pub crafting_laws:    CraftingRules,

    /// Respawn configuration.
    pub respawn_hub:      RespawnPolicy,
}

impl SafeCity {
    pub fn new(zone_id: impl Into<ZoneId>) -> Self {
        Self {
            zone_id:          zone_id.into(),
            protection_level: 1.0,
            allowed_actions:  AllowedAction::all(),
            market:           AuctionHouse::new(),
            crafting_laws:    CraftingRules::default(),
            respawn_hub:      RespawnPolicy::default(),
        }
    }

    /// True if the given action is permitted in this city.
    pub fn allows(&self, action: &AllowedAction) -> bool {
        self.allowed_actions.contains(action)
    }
}

// ─── AllowedAction ───────────────────────────────────────────────────────────

/// Actions permitted within the Safe City zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowedAction {
    Trade,
    Crafting,
    CraftFusion,
    SkillProgression,
    /// Players and AI may converse / negotiate.
    NpcInteraction,
    Respawn,
}

impl AllowedAction {
    pub fn all() -> Vec<AllowedAction> {
        vec![
            AllowedAction::Trade,
            AllowedAction::Crafting,
            AllowedAction::CraftFusion,
            AllowedAction::SkillProgression,
            AllowedAction::NpcInteraction,
            AllowedAction::Respawn,
        ]
    }
}

// ─── CraftingRules ───────────────────────────────────────────────────────────

/// Rules governing crafting inside the Safe City.
///
/// Prevents economic exploits from crafting loops.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftingRules {
    /// Maximum number of crafts per player per world tick.
    pub max_crafts_per_tick: u32,

    /// Whether AI factions can craft here.
    pub ai_crafting_allowed: bool,

    /// Minimum level required to use the fusion system.
    pub fusion_min_level: u32,
}

impl Default for CraftingRules {
    fn default() -> Self {
        Self {
            max_crafts_per_tick: 5,
            ai_crafting_allowed: true,
            fusion_min_level:    10,
        }
    }
}

// ─── RespawnPolicy ───────────────────────────────────────────────────────────

/// Respawn configuration for player clones.
///
/// Players always respawn at the Safe City if clone charges remain.
/// No respawn is possible after all clone charges are consumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespawnPolicy {
    /// Number of clone charges each player starts a run with.
    pub initial_clone_charges: u32,

    /// Skill decay rate on respawn (0.0–1.0).
    pub skill_decay_on_death: f32,

    /// Fraction of inventory secured in the city vault on death.
    pub vault_fraction: f32,
}

impl Default for RespawnPolicy {
    fn default() -> Self {
        Self {
            initial_clone_charges: 3,
            skill_decay_on_death:  0.05,
            vault_fraction:        0.50,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_city_allows_trade() {
        let city = SafeCity::new("hub-zone");
        assert!(city.allows(&AllowedAction::Trade));
        assert!(city.allows(&AllowedAction::Respawn));
    }

    #[test]
    fn safe_city_does_not_allow_combat() {
        let city = SafeCity::new("hub-zone");
        // Combat is not in AllowedAction — safe city is peaceful.
        let all = AllowedAction::all();
        assert!(!all.iter().any(|a| format!("{a:?}").to_lowercase().contains("combat")));
    }

    #[test]
    fn default_crafting_rules_sensible() {
        let r = CraftingRules::default();
        assert!(r.max_crafts_per_tick > 0);
        assert!(r.ai_crafting_allowed);
    }
}
