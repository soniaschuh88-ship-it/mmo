//! Animation graph compiler — text → [`AnimationGraphIR`].

use crate::types::*;
use crate::validate::WacError;

/// Standard state keyword → state configuration mapping.
const STATE_MAP: &[(&str, u32, bool)] = &[
    // (keyword, duration_ms, is_loop)
    ("idle",   2000, true),
    ("walk",    600, true),
    ("run",     400, true),
    ("patrol",  800, true),
    ("search",  500, true),
    ("attack",  400, false),
    ("cast",    700, false),
    ("dodge",   250, false),
    ("stun",    600, false),
    ("flee",    450, true),
    ("speak",  1200, false),
    ("die",     800, false),
    ("spawn",   600, false),
];

/// Standard transition rules by state pair.
/// (from, to, condition, priority)
const TRANSITION_RULES: &[(&str, &str, &str, u32)] = &[
    ("idle",    "search",  "player_near",      10),
    ("idle",    "patrol",  "timeout",           1),
    ("patrol",  "search",  "player_near",      10),
    ("patrol",  "idle",    "timeout",           1),
    ("search",  "attack",  "enemy_visible",    20),
    ("search",  "idle",    "target_lost",       5),
    ("attack",  "flee",    "health_below_20",  30),
    ("attack",  "idle",    "target_lost",       5),
    ("flee",    "idle",    "health_above_40",  15),
    ("flee",    "idle",    "target_lost",       5),
    ("stun",    "idle",    "timeout",          40),
    ("speak",   "idle",    "timeout",          10),
    ("cast",    "idle",    "timeout",           5),
    ("any",     "die",     "on_death",        100),
    ("any",     "idle",    "on_respawn",      100),
];

pub fn compile(bp: &crate::types::AssetBlueprint) -> Result<AnimationGraphIR, WacError> {
    let spec = bp.natural_language_spec.to_lowercase();

    // ── Extract states from spec ──────────────────────────────────────────────

    let mut states: Vec<AnimState> = Vec::new();

    // Always include idle + die (required by engine)
    states.push(AnimState { id: "idle".into(), is_loop: true,  duration_ms: 2000 });
    states.push(AnimState { id: "die".into(),  is_loop: false, duration_ms:  800 });
    states.push(AnimState { id: "spawn".into(),is_loop: false, duration_ms:  600 });

    for &(keyword, dur, looping) in STATE_MAP {
        if keyword == "idle" || keyword == "die" || keyword == "spawn" { continue; }
        if spec.contains(keyword) {
            states.push(AnimState { id: keyword.into(), is_loop: looping, duration_ms: dur });
        }
    }

    // Deduplicate (shouldn't happen but guard anyway)
    states.dedup_by(|a, b| a.id == b.id);

    let state_ids: Vec<&str> = states.iter().map(|s| s.id.as_str()).collect();

    // ── Generate transitions ──────────────────────────────────────────────────

    let mut transitions: Vec<AnimTransition> = Vec::new();

    for &(from, to, cond_str, priority) in TRANSITION_RULES {
        // "any" means from every existing state except the target.
        let sources: Vec<&str> = if from == "any" {
            state_ids.iter().filter(|&&s| s != to).copied().collect()
        } else if state_ids.contains(&from) {
            vec![from]
        } else {
            continue;
        };

        if !state_ids.contains(&to) { continue; }

        let condition = parse_condition(cond_str, bp.seed)?;

        for src in sources {
            transitions.push(AnimTransition {
                from:      src.into(),
                to:        to.into(),
                condition: condition.clone(),
                priority,
            });
        }
    }

    // Sort by priority (highest = evaluated first)
    transitions.sort_by(|a, b| b.priority.cmp(&a.priority));

    // Entity type from spec (first recognizable word)
    let entity_type = extract_entity_type(&spec);

    Ok(AnimationGraphIR {
        id: format!("anim_{entity_type}"),
        entity_type,
        states,
        transitions,
    })
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn parse_condition(s: &str, seed: u64) -> Result<TransitionCondition, WacError> {
    Ok(match s {
        "player_near"     => TransitionCondition::PlayerNear  { radius: 10.0 + (seed % 5) as f32 },
        "enemy_visible"   => TransitionCondition::EnemyVisible,
        "health_below_20" => TransitionCondition::HealthBelow { fraction: 0.20 },
        "health_above_40" => TransitionCondition::HealthAbove { fraction: 0.40 },
        "target_lost"     => TransitionCondition::TargetLost,
        "timeout"         => TransitionCondition::Timeout     { ms: 3000 },
        "on_death"        => TransitionCondition::OnDeath,
        "on_respawn"      => TransitionCondition::OnRespawn,
        "attack_hit"      => TransitionCondition::AttackHit,
        other             => return Err(WacError::CompilerError(format!("unknown condition: {other}"))),
    })
}

fn extract_entity_type(spec: &str) -> String {
    for word in ["wolf","goblin","bat","skeleton","rat","boss","npc","mob","creature","entity"] {
        if spec.contains(word) { return word.into(); }
    }
    "generic".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssetBlueprint, AssetIntent};

    fn anim_bp(spec: &str) -> AssetBlueprint {
        AssetBlueprint::new(AssetIntent::AnimationGraph, spec, vec![], 1)
    }

    #[test]
    fn idle_always_present() {
        let ir = compile(&anim_bp("idle search attack flee")).unwrap();
        assert!(ir.states.iter().any(|s| s.id == "idle"));
    }

    #[test]
    fn die_always_present() {
        let ir = compile(&anim_bp("idle attack flee")).unwrap();
        assert!(ir.states.iter().any(|s| s.id == "die"));
    }

    #[test]
    fn flee_included_when_in_spec() {
        let ir = compile(&anim_bp("idle search attack flee")).unwrap();
        assert!(ir.states.iter().any(|s| s.id == "flee"));
    }

    #[test]
    fn transitions_have_correct_direction() {
        let ir = compile(&anim_bp("idle search attack flee")).unwrap();
        // attack → flee should exist
        assert!(ir.transitions.iter().any(|t| t.from == "attack" && t.to == "flee"));
    }

    #[test]
    fn deterministic() {
        let bp  = anim_bp("idle patrol search attack flee");
        let ir1 = compile(&bp).unwrap();
        let ir2 = compile(&bp).unwrap();
        assert_eq!(ir1, ir2);
    }
}
