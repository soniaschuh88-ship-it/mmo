//! Biome compiler — text → [`BiomeIR`].

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::types::*;
use crate::validate::WacError;
use super::{has, make_id, title_case};

/// Keyword sets for terrain classification.
#[allow(dead_code)]
const FOREST_KEYS:   &[&str] = &["forest","wald","tree","trees","wood","jungle","grove"];
const DESERT_KEYS:   &[&str] = &["desert","sand","arid","dry","sahara","wüste"];
const SNOW_KEYS:     &[&str] = &["snow","ice","frozen","arctic","tundra","schnee","frost"];
const CRYSTAL_KEYS:  &[&str] = &["crystal","kristall","gem","glowing","luminous","quartz"];
const SWAMP_KEYS:    &[&str] = &["swamp","sumpf","bog","marsh","mud","murky"];
const MOUNTAIN_KEYS: &[&str] = &["mountain","peak","cliff","volcanic","gebirge","berg","rock"];
const OCEAN_KEYS:    &[&str] = &["ocean","sea","water","coastal","beach","ozean"];
const PLAINS_KEYS:   &[&str] = &["plains","grass","savanna","field","meadow","valley"];

/// Night/glow keywords.
const NIGHT_KEYS:    &[&str] = &["night","nocturnal","glow","glowing","leuchten","luminous","nacht"];

/// Hostility keywords.
const HOSTILE_KEYS:  &[&str] = &["hostile","aggressive","dangerous","aggro","enemy","feindlich"];

pub fn compile(bp: &crate::types::AssetBlueprint) -> Result<BiomeIR, WacError> {
    let spec  = bp.natural_language_spec.to_lowercase();
    let mut rng = StdRng::seed_from_u64(bp.seed);

    // ── Classify terrain ──────────────────────────────────────────────────────

    #[derive(Clone, Copy)]
    enum Terrain { Forest, Desert, Snow, Crystal, Swamp, Mountain, Ocean, Plains }

    let terrain = if   has(&spec, CRYSTAL_KEYS)  { Terrain::Crystal  }
                  else if has(&spec, SNOW_KEYS)   { Terrain::Snow     }
                  else if has(&spec, DESERT_KEYS) { Terrain::Desert   }
                  else if has(&spec, SWAMP_KEYS)  { Terrain::Swamp    }
                  else if has(&spec, OCEAN_KEYS)  { Terrain::Ocean    }
                  else if has(&spec, MOUNTAIN_KEYS){ Terrain::Mountain }
                  else if has(&spec, PLAINS_KEYS) { Terrain::Plains   }
                  else                            { Terrain::Forest   };

    let has_glow   = has(&spec, NIGHT_KEYS);
    let is_hostile = has(&spec, HOSTILE_KEYS);

    // ── Map terrain to base parameters ───────────────────────────────────────

    #[allow(clippy::match_like_matches_macro)]
    let (temp, humid, elev, tree_density,
         dominant, secondary, accent,
         ambient_color) = match terrain {
        Terrain::Forest   => (0.55, 0.70, 0.30, jitter(&mut rng, 0.70, 0.15),
            "oak_wood", "mossy_stone", "fern",         "#1a2a14"),
        Terrain::Desert   => (0.90, 0.05, 0.20, jitter(&mut rng, 0.02, 0.02),
            "sand", "sandstone", "dead_shrub",         "#4a3810"),
        Terrain::Snow     => (0.05, 0.40, 0.50, jitter(&mut rng, 0.20, 0.10),
            "snow", "ice", "frost_pine",               "#2a3a4a"),
        Terrain::Crystal  => (0.40, 0.30, 0.60, jitter(&mut rng, 0.40, 0.20),
            "crystal_red", "obsidian_black", "quartz", "#200a30"),
        Terrain::Swamp    => (0.65, 0.90, 0.10, jitter(&mut rng, 0.50, 0.20),
            "mud", "mossy_log", "hanging_vine",        "#141e0a"),
        Terrain::Mountain => (0.20, 0.25, 0.85, jitter(&mut rng, 0.15, 0.10),
            "granite", "gravel", "alpine_shrub",       "#2a2a2a"),
        Terrain::Ocean    => (0.60, 1.00, 0.00, jitter(&mut rng, 0.00, 0.00),
            "water", "sand", "seaweed",                "#0a1a2a"),
        Terrain::Plains   => (0.55, 0.45, 0.15, jitter(&mut rng, 0.10, 0.08),
            "grass", "dirt", "wildflower",             "#142a0a"),
    };

    // ── Light emission ────────────────────────────────────────────────────────

    let light_emission = if has_glow {
        Some(LightEmission {
            pattern:   if has(&spec, NIGHT_KEYS) { EmissionPattern::NocturnalGlow } else { EmissionPattern::SineFlicker },
            intensity: jitter(&mut rng, 0.65, 0.20),
            color:     if has(&spec, CRYSTAL_KEYS) { "#ff60a0".into() } else { "#60ffa0".into() },
        })
    } else {
        None
    };

    // ── Entity spawns ─────────────────────────────────────────────────────────

    let faction = if is_hostile { "hostile" } else { "neutral" };
    let entity_id = derive_entity_id(&spec, terrain);
    let time_cond = if has_glow { Some(TimeCondition::Night) } else { Some(TimeCondition::Always) };

    let entity_spawns = vec![
        SpawnRule {
            entity_id:      entity_id.clone(),
            density:        jitter(&mut rng, if is_hostile { 0.8 } else { 0.3 }, 0.2),
            time_condition: time_cond,
            min_elevation:  None,
            faction:        faction.into(),
        },
    ];

    // ── Biome id from spec slug ────────────────────────────────────────────────

    let id = make_id(&spec);
    let display_name = title_case(&id.replace('-', " "));

    Ok(BiomeIR {
        id: id.clone(),
        display_name,
        temperature:       apply_jitter(&mut rng, temp,  0.05),
        humidity:          apply_jitter(&mut rng, humid, 0.05),
        elevation:         apply_jitter(&mut rng, elev,  0.05),
        tree_density:      tree_density.clamp(0.0, 1.0),
        dominant_material: dominant.into(),
        secondary_material: secondary.into(),
        accent_material:   accent.into(),
        light_emission,
        ambient_color:     ambient_color.into(),
        entity_spawns,
        loot_distribution: LootGraphRef { loot_table_id: format!("lt_{id}") },
    })
}

// ─── Helpers ──────────────────────────────────────────────────────────────────
// `has`, `make_id`, `title_case` are shared helpers from `compile/mod.rs`.

/// Apply a uniform jitter of ±`range` around `base` using seeded RNG.
fn jitter(rng: &mut StdRng, base: f32, range: f32) -> f32 {
    (base + rng.gen_range(-range..=range)).clamp(0.0, 1.0)
}

fn apply_jitter(rng: &mut StdRng, base: f32, range: f32) -> f32 {
    jitter(rng, base, range)
}

/// Derive the dominant entity from the spec text.
fn derive_entity_id(spec: &str, _terrain: impl Copy) -> String {
    if spec.contains("bat")  { return "bat".into(); }
    if spec.contains("wolf") || spec.contains("wolf") { return "wolf".into(); }
    if spec.contains("goblin") { return "goblin".into(); }
    if spec.contains("skeleton") { return "skeleton".into(); }
    if spec.contains("rat") { return "rat".into(); }
    // Default per terrain
    match () {
        _ if spec.contains("crystal") => "crystal_bat",
        _ if spec.contains("forest")  => "wolf",
        _ if spec.contains("desert")  => "sand_worm",
        _ if spec.contains("snow")    => "ice_troll",
        _ if spec.contains("mountain")=> "goblin",
        _ => "generic_mob",
    }.into()
}

// `make_id` and `title_case` are imported from `compile/mod.rs` (see `use super::{…}`).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssetBlueprint, AssetIntent};

    fn biome_bp(spec: &str) -> AssetBlueprint {
        AssetBlueprint::new(AssetIntent::BiomeDefinition, spec, vec![], 42)
    }

    #[test]
    fn crystal_forest_biome() {
        let bp  = biome_bp("crystal forest with glowing trees at night");
        let ir  = compile(&bp).unwrap();
        assert_eq!(ir.dominant_material, "crystal_red");
        assert!(ir.light_emission.is_some());
        assert_eq!(ir.light_emission.as_ref().unwrap().pattern, EmissionPattern::NocturnalGlow);
    }

    #[test]
    fn desert_biome_low_trees() {
        let ir = compile(&biome_bp("hot sandy desert with scarce life")).unwrap();
        assert_eq!(ir.dominant_material, "sand");
        assert!(ir.tree_density < 0.15);
    }

    #[test]
    fn hostile_biome_has_hostile_spawn() {
        let ir = compile(&biome_bp("aggressive hostile forest with wolves")).unwrap();
        assert_eq!(ir.entity_spawns[0].faction, "hostile");
    }

    #[test]
    fn deterministic_same_seed() {
        let bp  = biome_bp("snow mountain tundra with ice trolls");
        let ir1 = compile(&bp).unwrap();
        let ir2 = compile(&bp).unwrap();
        assert_eq!(ir1, ir2);
    }

    #[test]
    fn different_seed_different_result() {
        let spec = "forest with wolves";
        let bp1  = AssetBlueprint::new(AssetIntent::BiomeDefinition, spec, vec![], 1);
        let bp2  = AssetBlueprint::new(AssetIntent::BiomeDefinition, spec, vec![], 2);
        let ir1  = compile(&bp1).unwrap();
        let ir2  = compile(&bp2).unwrap();
        // UUIDs differ but some float fields will differ due to RNG divergence
        assert_ne!(ir1.temperature, ir2.temperature);
    }
}
