//! Validation layer — checks every [`AssetBlueprint`] before it reaches the compiler.
//!
//! Rules (from WAC.md):
//! - seed must never be zero
//! - spec must be non-empty
//! - per-asset-type constraints are enforced here, not in the compilers

use thiserror::Error;

use crate::types::{AssetBlueprint, AssetIntent};

#[derive(Debug, Error, PartialEq)]
pub enum WacError {
    #[error("seed must not be zero (blueprint id={id})")]
    ZeroSeed { id: String },

    #[error("natural_language_spec must not be empty")]
    EmptySpec,

    #[error("invalid biome rule: {0}")]
    InvalidBiomeRule(String),

    #[error("loot constraint violated: {0}")]
    LootConstraintViolated(String),

    #[error("animation graph requires at least one state")]
    AnimationNoStates,

    #[error("animation graph must include an 'idle' state")]
    AnimationMissingIdle,

    #[error("entity prefab spec must mention at least one stat keyword (hp/atk/def/boss/mob)")]
    EntityNoStats,

    #[error("constraint parse error: {0}")]
    ConstraintParse(String),

    #[error("compiler error: {0}")]
    CompilerError(String),
}

/// Validate a blueprint before compilation.
///
/// Returns `Ok(())` if the blueprint is acceptable, or an error describing
/// the first violation found.
pub fn validate(bp: &AssetBlueprint) -> Result<(), WacError> {
    // Universal rules
    if bp.seed == 0 {
        return Err(WacError::ZeroSeed { id: bp.id.to_string() });
    }
    if bp.natural_language_spec.trim().is_empty() {
        return Err(WacError::EmptySpec);
    }

    // Per-type rules
    match bp.asset_type {
        AssetIntent::BiomeDefinition => validate_biome(bp),
        AssetIntent::LootTable       => validate_loot(bp),
        AssetIntent::AnimationGraph  => validate_animation(bp),
        AssetIntent::EntityPrefab    => validate_entity(bp),
        AssetIntent::VoxelStructure  => Ok(()), // structural rules checked at physics level
    }
}

// ─── Biome ────────────────────────────────────────────────────────────────────

fn validate_biome(bp: &AssetBlueprint) -> Result<(), WacError> {
    // Must contain at least one terrain keyword
    let spec_lower = bp.natural_language_spec.to_lowercase();
    let terrain_words = ["forest","desert","snow","crystal","swamp","ocean","mountain",
                         "cave","jungle","tundra","volcanic","plains","valley","wald",
                         "wüste","schnee","sumpf","gebirge"];
    if !terrain_words.iter().any(|w| spec_lower.contains(w)) {
        return Err(WacError::InvalidBiomeRule(
            "spec must contain at least one terrain keyword (forest, desert, snow, …)".into(),
        ));
    }
    // Validate constraint syntax for known constraint prefixes
    for c in &bp.constraints {
        if c.starts_with("max_drop_rate") {
            parse_float_constraint(c, "max_drop_rate")?;
        }
    }
    Ok(())
}

// ─── Loot ─────────────────────────────────────────────────────────────────────

fn validate_loot(bp: &AssetBlueprint) -> Result<(), WacError> {
    // Check max_drop_rate constraints
    for c in &bp.constraints {
        if c.starts_with("max_drop_rate") {
            let v = parse_float_constraint(c, "max_drop_rate")?;
            if !(0.0..=1.0).contains(&v) {
                return Err(WacError::LootConstraintViolated(
                    format!("max_drop_rate {v} must be between 0.0 and 1.0"),
                ));
            }
        }
    }
    Ok(())
}

// ─── Animation ────────────────────────────────────────────────────────────────

fn validate_animation(bp: &AssetBlueprint) -> Result<(), WacError> {
    let spec_lower = bp.natural_language_spec.to_lowercase();
    // Must list at least one state
    let state_keywords = ["idle","walk","run","attack","search","flee","die","spawn",
                          "patrol","speak","dodge","cast","stun"];
    if !state_keywords.iter().any(|s| spec_lower.contains(s)) {
        return Err(WacError::AnimationNoStates);
    }
    // Must include idle (required by engine)
    if !spec_lower.contains("idle") {
        return Err(WacError::AnimationMissingIdle);
    }
    Ok(())
}

// ─── Entity ───────────────────────────────────────────────────────────────────

fn validate_entity(bp: &AssetBlueprint) -> Result<(), WacError> {
    let spec_lower = bp.natural_language_spec.to_lowercase();
    let stat_words = ["hp","atk","def","boss","mob","npc","guard","monster","creature",
                      "enemy","friendly","neutral","hostile","strong","weak","fast"];
    if !stat_words.iter().any(|w| spec_lower.contains(w)) {
        return Err(WacError::EntityNoStats);
    }
    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Parse a constraint of the form `"key <= value"` or `"key = value"`.
fn parse_float_constraint(c: &str, key: &str) -> Result<f32, WacError> {
    let part = c
        .trim_start_matches(key)
        .trim_start_matches(|ch: char| ch == '<' || ch == '>' || ch == '=' || ch == ' ');
    part.trim().parse::<f32>().map_err(|_| {
        WacError::ConstraintParse(format!("cannot parse float from constraint: '{c}'"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssetBlueprint, AssetIntent};

    fn bp(intent: AssetIntent, spec: &str, seed: u64) -> AssetBlueprint {
        AssetBlueprint::new(intent, spec, vec![], seed)
    }

    #[test]
    fn zero_seed_rejected() {
        let b = bp(AssetIntent::BiomeDefinition, "forest with wolves", 0);
        assert!(matches!(validate(&b), Err(WacError::ZeroSeed { .. })));
    }

    #[test]
    fn empty_spec_rejected() {
        let b = bp(AssetIntent::BiomeDefinition, "   ", 1);
        assert_eq!(validate(&b), Err(WacError::EmptySpec));
    }

    #[test]
    fn biome_requires_terrain_keyword() {
        let b = bp(AssetIntent::BiomeDefinition, "very spooky place", 42);
        assert!(matches!(validate(&b), Err(WacError::InvalidBiomeRule(_))));
    }

    #[test]
    fn valid_biome() {
        let b = bp(AssetIntent::BiomeDefinition, "crystal forest with glowing trees", 42);
        assert!(validate(&b).is_ok());
    }

    #[test]
    fn loot_rate_out_of_range() {
        let b = AssetBlueprint::new(
            AssetIntent::LootTable, "crystals from bats",
            vec!["max_drop_rate <= 1.5".into()], 7,
        );
        assert!(matches!(validate(&b), Err(WacError::LootConstraintViolated(_))));
    }

    #[test]
    fn animation_missing_idle_rejected() {
        let b = bp(AssetIntent::AnimationGraph, "attack flee patrol", 1);
        assert_eq!(validate(&b), Err(WacError::AnimationMissingIdle));
    }

    #[test]
    fn valid_animation() {
        let b = bp(AssetIntent::AnimationGraph, "idle search attack flee", 1);
        assert!(validate(&b).is_ok());
    }

    #[test]
    fn valid_entity() {
        let b = bp(AssetIntent::EntityPrefab, "hostile mob with high hp and fast atk", 99);
        assert!(validate(&b).is_ok());
    }
}
