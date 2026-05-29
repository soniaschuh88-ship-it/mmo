//! Loot table compiler — text → [`LootTableIR`].

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::types::*;
use crate::validate::WacError;
use super::{has, title_case};

/// Rarity keywords → base drop rate ranges.
const RARE_KEYS:   &[&str] = &["rare","selten","very rare","ultra","legendary"];
const UNCOMMON_KEYS: &[&str] = &["uncommon","occasional","sometimes","sometimes"];
const COMMON_KEYS: &[&str] = &["common","frequent","often","always","basic"];

/// Time condition keywords.
const NIGHT_KEYS:  &[&str] = &["night","nocturnal","nacht","darkness"];
const DAY_KEYS:    &[&str] = &["day","daylight","daytime","tag"];

pub fn compile(bp: &crate::types::AssetBlueprint) -> Result<LootTableIR, WacError> {
    let spec  = bp.natural_language_spec.to_lowercase();
    let mut rng = StdRng::seed_from_u64(bp.seed);

    // ── Extract max drop rate from constraints ────────────────────────────────

    let mut max_rate: f32 = 1.0;
    for c in &bp.constraints {
        if c.starts_with("max_drop_rate") {
            let v = parse_rate(c)?;
            max_rate = max_rate.min(v);
        }
    }

    // ── Determine rarity ──────────────────────────────────────────────────────

    let base_rate = if has(&spec, RARE_KEYS) {
        rng.gen_range(0.01..=0.05_f32)
    } else if has(&spec, UNCOMMON_KEYS) {
        rng.gen_range(0.08..=0.15_f32)
    } else if has(&spec, COMMON_KEYS) {
        rng.gen_range(0.25..=0.45_f32)
    } else {
        rng.gen_range(0.05..=0.20_f32)
    };
    let drop_rate = base_rate.min(max_rate);

    // ── Item name extraction ──────────────────────────────────────────────────

    let item_id = extract_item_id(&spec);

    // ── Drop conditions ───────────────────────────────────────────────────────

    let mut conditions: Vec<DropCondition> = Vec::new();

    if has(&spec, NIGHT_KEYS) {
        conditions.push(DropCondition::Night);
    } else if has(&spec, DAY_KEYS) {
        conditions.push(DropCondition::Day);
    }

    // Entity condition: "from bats", "from wolves", etc.
    if let Some(entity) = extract_entity(&spec) {
        conditions.push(DropCondition::KillType { entity_id: entity });
    }

    // Biome condition: "in crystal forest", "in dungeon", etc.
    if let Some(biome) = extract_biome(&spec) {
        conditions.push(DropCondition::InBiome { biome_id: biome });
    }

    // ── Quantities ────────────────────────────────────────────────────────────

    let (min_qty, max_qty) = if has(&spec, RARE_KEYS) { (1, 1) }
                             else if has(&spec, COMMON_KEYS) { (1, rng.gen_range(2..=4u32)) }
                             else { (1, rng.gen_range(1..=2u32)) };

    let id           = format!("lt_{}", sanitize(&spec, 3));
    let display_name = format!("{} Loot Table", title_case(&item_id));

    Ok(LootTableIR {
        id,
        display_name,
        entries: vec![LootEntry { item_id, base_rate: drop_rate, min_qty, max_qty, conditions }],
    })
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

// `has` and `title_case` are imported from `compile/mod.rs`.

fn extract_item_id(spec: &str) -> String {
    // Look for common item nouns
    let items = ["crystal","shard","pelt","fur","bone","tooth","claw","fang","coin",
                 "gem","ore","herb","scroll","tome","potion","key","fragment","scale",
                 "kristall","scherbe","fell","knochen","zahn","klaue","münze"];
    for item in &items {
        if spec.contains(item) { return item.to_string(); }
    }
    "material".into()
}

fn extract_entity(spec: &str) -> Option<String> {
    let from_idx = spec.find(" from ")?;
    let tail = &spec[from_idx + 6..];
    let entity = tail.split_whitespace().next()?.trim_matches(|c: char| !c.is_alphanumeric());
    if entity.is_empty() { None } else { Some(entity.into()) }
}

fn extract_biome(spec: &str) -> Option<String> {
    let in_idx = spec.find(" in ")?;
    let tail = &spec[in_idx + 4..];
    // Take up to 2 words
    let biome: String = tail.split_whitespace().take(2)
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .collect::<Vec<_>>().join("_");
    if biome.is_empty() { None } else { Some(biome) }
}

fn parse_rate(c: &str) -> Result<f32, WacError> {
    let part = c.trim_start_matches(|ch: char| ch.is_alphabetic() || ch == '_')
                .trim_start_matches(|ch: char| ch == '<' || ch == '>' || ch == '=' || ch == ' ');
    part.trim().parse::<f32>()
        .map_err(|_| WacError::ConstraintParse(format!("cannot parse: '{c}'")))
}

fn sanitize(spec: &str, words: usize) -> String {
    spec.split_whitespace().take(words)
        .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase())
        .collect::<Vec<_>>().join("_")
}

// `title_case` is imported from `compile/mod.rs`.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssetBlueprint, AssetIntent};

    fn loot_bp(spec: &str, constraints: Vec<String>) -> AssetBlueprint {
        AssetBlueprint::new(AssetIntent::LootTable, spec, constraints, 7)
    }

    #[test]
    fn rare_crystal_drop_low_rate() {
        let bp = loot_bp("rare crystals drop from bats at night", vec![]);
        let ir = compile(&bp).unwrap();
        assert_eq!(ir.entries.len(), 1);
        assert!(ir.entries[0].base_rate <= 0.05);
    }

    #[test]
    fn max_rate_constraint_respected() {
        let bp = loot_bp("common crystals from wolves", vec!["max_drop_rate <= 0.10".into()]);
        let ir = compile(&bp).unwrap();
        assert!(ir.entries[0].base_rate <= 0.10 + f32::EPSILON);
    }

    #[test]
    fn night_condition_attached() {
        let bp = loot_bp("rare crystals from glowing bats at night", vec![]);
        let ir = compile(&bp).unwrap();
        assert!(ir.entries[0].conditions.contains(&DropCondition::Night));
    }

    #[test]
    fn deterministic() {
        let bp  = loot_bp("rare crystals from bats at night", vec![]);
        let ir1 = compile(&bp).unwrap();
        let ir2 = compile(&bp).unwrap();
        assert_eq!(ir1, ir2);
    }
}
