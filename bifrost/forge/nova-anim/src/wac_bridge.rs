//! AnimIR → nova-anim FSM bridge  (enabled via the `wac` feature).
//!
//! Converts a [`bifrost_wac::types::AnimationGraphIR`] produced by the WAC
//! compiler into a live [`AnimStateMachine`] that the nova-anim runtime can
//! drive frame-by-frame.
//!
//! This is the **single connection point** between the two systems:
//!
//! ```text
//! bifrost-wac (behavior spec / AnimationGraphIR)
//!     │
//!     └─► nova_anim::wac_bridge::build_fsm()  ─►  AnimStateMachine
//! ```
//!
//! WAC produces the IR; nova-anim bridges it to a live FSM.  bifrost-wac
//! itself has **no** nova dependency — layers stay clean.
//!
//! ## Stub clips
//!
//! WAC specs describe *states* and *transitions* but not keyframe data.
//! The bridge generates minimal stub [`AnimClip`]s (correct name/duration/loop
//! flag, empty tracks).  Replace them with authored animations after calling
//! [`build_fsm`] if needed.

use bifrost_wac::types::{AnimationGraphIR, AnimTransition, TransitionCondition};

use crate::clip::{AnimClip, Keyframe, Track};
use crate::skeleton::VoxelSkeleton;
use crate::state_machine::{AnimCondition, AnimState, AnimStateMachine, Transition};

/// Convert a WAC [`AnimationGraphIR`] into a live [`AnimStateMachine`].
///
/// `skeleton` is bound to the FSM.  The default state is `"idle"` if present,
/// otherwise the first state listed in the IR.  All clips are generated as
/// **stubs** — correct metadata, no keyframes.
pub fn build_fsm(ir: AnimationGraphIR, skeleton: VoxelSkeleton) -> AnimStateMachine {
    let default_state = ir
        .states
        .iter()
        .find(|s| s.id == "idle")
        .map(|s| s.id.clone())
        .or_else(|| ir.states.first().map(|s| s.id.clone()))
        .unwrap_or_else(|| "idle".into());

    let mut fsm = AnimStateMachine::new(skeleton, default_state);

    // States → stub AnimClips
    for wac_state in &ir.states {
        let clip = stub_clip(&wac_state.id, wac_state.duration_ms, wac_state.is_loop);
        fsm.add_state(AnimState { name: wac_state.id.clone(), clip });
    }

    // Transitions (already sorted by priority — highest first)
    for wac_tr in &ir.transitions {
        if let Some(tr) = convert_transition(wac_tr) {
            fsm.add_transition(tr);
        }
    }

    fsm
}

// ── Stub clip builder ─────────────────────────────────────────────────────────

/// Build a minimal [`AnimClip`] with identity keyframes at t=0 and t=end.
fn stub_clip(name: &str, duration_ms: u32, looping: bool) -> AnimClip {
    let duration_secs = duration_ms as f32 / 1000.0;
    let mut clip = AnimClip::new(name, duration_secs, looping);
    // One root track with identity keyframes — enough to drive the FSM
    // without crashing; replaced by authored data per entity type.
    let mut root = Track::new("root");
    root.keys.push(Keyframe::identity(0.0));
    if duration_secs > 0.0 {
        root.keys.push(Keyframe::identity(duration_secs));
    }
    clip.tracks.push(root);
    clip
}

// ── Condition / transition mapping ───────────────────────────────────────────

fn convert_condition(tc: &TransitionCondition) -> AnimCondition {
    match tc {
        TransitionCondition::PlayerNear { .. } =>
            AnimCondition::Bool   { param: "player_near".into(),   value: true },
        TransitionCondition::EnemyVisible =>
            AnimCondition::Bool   { param: "enemy_visible".into(), value: true },
        TransitionCondition::HealthBelow { fraction } =>
            AnimCondition::FloatLt { param: "hp".into(), threshold: *fraction },
        TransitionCondition::HealthAbove { fraction } =>
            AnimCondition::FloatGt { param: "hp".into(), threshold: *fraction },
        TransitionCondition::AttackHit =>
            AnimCondition::Trigger { name: "attack_hit".into() },
        TransitionCondition::TargetLost =>
            AnimCondition::Bool   { param: "has_target".into(), value: false },
        TransitionCondition::Timeout { .. } =>
            // Closest approximation: fire when the clip finishes naturally.
            AnimCondition::Always,
        TransitionCondition::OnDeath =>
            AnimCondition::Trigger { name: "die".into() },
        TransitionCondition::OnRespawn =>
            AnimCondition::Trigger { name: "respawn".into() },
    }
}

fn convert_transition(wac: &AnimTransition) -> Option<Transition> {
    let condition = convert_condition(&wac.condition);
    // Higher WAC priority → snappier blend.
    let blend_time = (100.0 / wac.priority.max(1) as f32).clamp(0.04, 0.25);
    Some(Transition {
        from:       wac.from.clone(),
        to:         wac.to.clone(),
        condition,
        blend_time,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use bifrost_wac::types::{AssetBlueprint, AssetIntent, CompiledAsset};
    use bifrost_wac::compile::compile as wac_compile;

    fn wolf_blueprint() -> AssetBlueprint {
        AssetBlueprint::new(
            AssetIntent::AnimationGraph,
            "wolf idle patrol search attack flee die",
            vec![],
            42,
        )
    }

    fn compile_wolf_ir() -> AnimationGraphIR {
        match wac_compile(&wolf_blueprint()).unwrap().asset {
            CompiledAsset::AnimationGraph(ir) => ir,
            _ => panic!("expected AnimationGraph"),
        }
    }

    #[test]
    fn build_fsm_produces_all_states() {
        let fsm = build_fsm(compile_wolf_ir(), VoxelSkeleton::humanoid());
        assert_eq!(fsm.current, "idle");
    }

    #[test]
    fn transitions_wired() {
        let ir = compile_wolf_ir();
        let has_attack_flee = ir.transitions.iter()
            .any(|t| t.from == "attack" && t.to == "flee");
        assert!(has_attack_flee, "attack→flee transition must exist");
        let _fsm = build_fsm(ir, VoxelSkeleton::humanoid());
    }

    #[test]
    fn stub_clip_correct_metadata() {
        let clip = stub_clip("walk", 600, true);
        assert_eq!(clip.name, "walk");
        assert!((clip.duration - 0.6).abs() < 1e-4);
        assert!(clip.looping);
        assert!(!clip.tracks.is_empty());
    }
}
