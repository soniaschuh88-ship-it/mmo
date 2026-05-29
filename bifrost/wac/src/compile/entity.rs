//! Entity prefab compiler — text → [`EntityPrefabIR`].

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::types::*;
use crate::validate::WacError;
use super::has;

/// Size tier keywords and stat multipliers.
const BOSS_KEYS:   &[&str] = &["boss","overlord","lord","king","queen","giant","titan","ancient"];
const STRONG_KEYS: &[&str] = &["strong","powerful","elite","champion","veteran","tough"];
const FAST_KEYS:   &[&str] = &["fast","quick","agile","swift","nimble","speed"];
const WEAK_KEYS:   &[&str] = &["weak","small","tiny","minion","grunt","basic"];

/// Faction keywords.
const HOSTILE_KEYS:  &[&str] = &["hostile","aggressive","enemy","feindlich","aggro","dangerous"];
const FRIENDLY_KEYS: &[&str] = &["friendly","peaceful","guard","npc","merchant","vendor","quest"];

pub fn compile(bp: &crate::types::AssetBlueprint) -> Result<EntityPrefabIR, WacError> {
    let spec  = bp.natural_language_spec.to_lowercase();
    let mut rng = StdRng::seed_from_u64(bp.seed);

    // ── Tier ──────────────────────────────────────────────────────────────────

    let is_boss     = has(&spec, BOSS_KEYS);
    let is_strong   = has(&spec, STRONG_KEYS);
    let is_fast     = has(&spec, FAST_KEYS);
    let is_weak     = has(&spec, WEAK_KEYS);

    // ── Faction ───────────────────────────────────────────────────────────────

    let (entity_class, faction) = if has(&spec, FRIENDLY_KEYS) {
        (EntityClass::NpcFriendly, "friendly".to_string())
    } else if has(&spec, HOSTILE_KEYS) || is_boss {
        if is_boss { (EntityClass::Boss, "hostile".to_string()) }
        else       { (EntityClass::Mob,  "hostile".to_string()) }
    } else {
        (EntityClass::NpcNeutral, "neutral".to_string())
    };

    // ── Base stat computation (deterministic via seed) ─────────────────────────

    let tier_mult: f32 = if is_boss     { 5.0 }
                         else if is_strong { 2.0 }
                         else if is_weak   { 0.5 }
                         else              { 1.0 };

    let hp  = ((rng.gen_range(20u32..=40u32) as f32) * tier_mult) as u32;
    let atk = ((rng.gen_range(5u32..=15u32)  as f32) * tier_mult * if is_fast { 0.7 } else { 1.0 }) as u32;
    let def = ((rng.gen_range(2u32..=8u32)   as f32) * tier_mult * if is_fast { 0.5 } else { 1.0 }) as u32;
    let xp  = ((rng.gen_range(10u32..=25u32) as f32) * tier_mult) as u32;
    let gold = ((rng.gen_range(2u32..=8u32)  as f32) * tier_mult) as u32;

    // ── Behaviour params ──────────────────────────────────────────────────────

    let aggro_range  = if matches!(entity_class, EntityClass::NpcFriendly | EntityClass::NpcNeutral) {
        0.0  // friendly/neutral NPCs don't aggro
    } else {
        rng.gen_range(8.0_f32..=18.0_f32) * if is_strong { 1.3 } else { 1.0 }
    };

    let flee_hp_frac = if is_boss { 0.0 } else { rng.gen_range(0.10_f32..=0.25_f32) };

    // ── Display ───────────────────────────────────────────────────────────────

    let entity_id    = extract_entity_id(&spec);
    let (icon, color) = icon_and_color(&spec, is_boss);

    let display_name = make_display_name(&spec, is_boss);

    Ok(EntityPrefabIR {
        id: entity_id.clone(),
        display_name,
        entity_class,
        hp_base:   hp,
        atk_base:  atk,
        def_base:  def,
        xp_reward: xp,
        gold_reward: gold,
        faction,
        aggro_range,
        leash_range: aggro_range * 3.0,
        flee_hp_frac,
        animation_graph_id: format!("anim_{entity_id}"),
        loot_table_id:      format!("lt_{entity_id}"),
        icon,
        color,
    })
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

// `has` is imported from `compile/mod.rs`.

fn extract_entity_id(spec: &str) -> String {
    let nouns = ["wolf","goblin","skeleton","rat","bat","troll","golem","dragon",
                 "guard","merchant","wizard","warrior","archer","knight",
                 "bandit","ogre","vampire","zombie","ghost","elemental"];
    for noun in &nouns {
        if spec.contains(noun) { return noun.to_string(); }
    }
    // Fallback: take first meaningful word
    spec.split_whitespace()
        .find(|w| w.len() >= 3 && !["the","a","an","with","has","very","high","low"].contains(w))
        .unwrap_or("entity")
        .chars().filter(|c| c.is_alphanumeric()).collect::<String>()
        .to_lowercase()
}

fn icon_and_color(spec: &str, is_boss: bool) -> (String, String) {
    if spec.contains("wolf")     { ("🐺".into(), "#8B5A2B".into()) }
    else if spec.contains("bat") { ("🦇".into(), "#4a2060".into()) }
    else if spec.contains("goblin")   { ("👺".into(), "#3a8a3a".into()) }
    else if spec.contains("skeleton") { ("💀".into(), "#c8c8b0".into()) }
    else if spec.contains("rat")      { ("🐀".into(), "#8a7060".into()) }
    else if spec.contains("dragon")   { ("🐉".into(), "#c03020".into()) }
    else if spec.contains("guard")    { ("⚔".into(),  "#a0a0b8".into()) }
    else if spec.contains("wizard")   { ("🧙".into(), "#9060e0".into()) }
    else if is_boss { ("👹".into(), "#a020a0".into()) }
    else            { ("👾".into(), "#808080".into()) }
}

fn make_display_name(spec: &str, is_boss: bool) -> String {
    let entity = extract_entity_id(spec);
    let mut name = capitalize(&entity);
    if is_boss { name = format!("{name} Overlord"); }
    name
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() { None => String::new(), Some(f) => f.to_uppercase().to_string() + c.as_str() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssetBlueprint, AssetIntent};

    fn entity_bp(spec: &str) -> AssetBlueprint {
        AssetBlueprint::new(AssetIntent::EntityPrefab, spec, vec![], 99)
    }

    #[test]
    fn boss_has_high_hp() {
        let ir = compile(&entity_bp("dungeon overlord boss with massive hp")).unwrap();
        assert!(ir.hp_base >= 80);
        assert_eq!(ir.flee_hp_frac, 0.0); // bosses never flee
    }

    #[test]
    fn friendly_npc_zero_aggro() {
        let ir = compile(&entity_bp("friendly guard npc with quest")).unwrap();
        assert_eq!(ir.aggro_range, 0.0);
        assert!(matches!(ir.entity_class, EntityClass::NpcFriendly));
    }

    #[test]
    fn wolf_gets_wolf_icon() {
        let ir = compile(&entity_bp("hostile wolf mob with fast atk")).unwrap();
        assert_eq!(ir.icon, "🐺");
    }

    #[test]
    fn deterministic() {
        let bp  = entity_bp("aggressive goblin mob with atk");
        let ir1 = compile(&bp).unwrap();
        let ir2 = compile(&bp).unwrap();
        assert_eq!(ir1, ir2);
    }
}
