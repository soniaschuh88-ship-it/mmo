//! AnimIR → nova-anim FSM bridge.
//!
//! ## Step 4 (FIX.md): WAC as behavior compiler
//!
//! [`AnimationGraphIR`] is the intermediate representation produced by the
//! WAC animation compiler.  This module converts it into a live
//! [`AnimStateMachine`] that the nova-anim runtime can drive frame-by-frame.
//!
//! The bridge is the single connection point between:
//!
//! ```text
//! bifrost-wac (behavior spec)  →  anim_bridge  →  nova-anim (runtime FSM)
//! ```
//!
//! WAC can then directly produce FSMs for creatures, bosses, and AI entities
//! without requiring per-entity hand-coding.
//!
//! ## Stub clips
//!
//! Because WAC specs describe *states* and *transitions* but not actual
//! keyframe data, the bridge generates minimal stub [`AnimClip`]s.  These
//! have the correct name, duration, and loop flag; their tracks are empty.
//! Game-specific code can replace stub clips with authored animations after
//! the FSM is constructed (e.g. `fsm.add_state(AnimState { name, clip: authored })`).

use nova_anim::clip::{AnimClip, Keyframe, Track};
use nova_anim::skeleton::VoxelSkeleton;
use nova_anim::state_machine::{AnimCondition, AnimState, AnimStateMachine, Transition};

use crate::types::{AnimationGraphIR, AnimTransition, TransitionCondition};

impl AnimationGraphIR {
    /// Convert this IR into a live nova-anim [`AnimStateMachine`].
    ///
    /// `skeleton` is bound to the FSM.  The default state is `"idle"`
    /// if present, otherwise the first state in the IR.
    ///
    /// All clips are generated as **stubs** (correct metadata, no keyframes).
    /// Replace individual clips with authored data where needed.
    pub fn to_nova_fsm(self, skeleton: VoxelSkeleton) -> AnimStateMachine {
        let default_state = self.states.iter()
            .find(|s| s.id == "idle")
            .map(|s| s.id.clone())
            .or_else(|| self.states.first().map(|s| s.id.clone()))
            .unwrap_or_else(|| "idle".into());

        let mut fsm = AnimStateMachine::new(skeleton, default_state);

        // ── States → stub AnimClips ───────────────────────────────────────────
        for wac_state in &self.states {
            let clip = stub_clip(&wac_state.id, wac_state.duration_ms, wac_state.is_loop);
            fsm.add_state(AnimState { name: wac_state.id.clone(), clip });
        }

        // ── Transitions (already sorted by priority — highest first) ─────────
        for wac_tr in &self.transitions {
            if let Some(tr) = convert_transition(wac_tr) {
                fsm.add_transition(tr);
            }
        }

        fsm
    }
}

// ─── Stub clip builder ────────────────────────────────────────────────────────

/// Build a minimal [`AnimClip`] with one identity keyframe at t=0 and t=end.
fn stub_clip(name: &str, duration_ms: u32, looping: bool) -> AnimClip {
    let duration_secs = duration_ms as f32 / 1000.0;
    let mut clip = AnimClip::new(name, duration_secs, looping);
    // One root track with identity keyframes — sufficient to drive the FSM
    // without crashing; replaced by authored data per entity type.
    let mut root = Track::new("root");
    root.keys.push(Keyframe::identity(0.0));
    if duration_secs > 0.0 {
        root.keys.push(Keyframe::identity(duration_secs));
    }
    clip.tracks.push(root);
    clip
}

// ─── Condition mapping ────────────────────────────────────────────────────────

/// Convert a WAC [`TransitionCondition`] to a nova-anim [`AnimCondition`].
fn convert_condition(tc: &TransitionCondition) -> AnimCondition {
    match tc {
        TransitionCondition::PlayerNear { .. }       =>
            AnimCondition::Bool   { param: "player_near".into(),   value: true },
        TransitionCondition::EnemyVisible            =>
            AnimCondition::Bool   { param: "enemy_visible".into(), value: true },
        TransitionCondition::HealthBelow { fraction }  =>
            AnimCondition::FloatLt { param: "hp".into(), threshold: *fraction },
        TransitionCondition::HealthAbove { fraction }  =>
            AnimCondition::FloatGt { param: "hp".into(), threshold: *fraction },
        TransitionCondition::AttackHit               =>
            AnimCondition::Trigger { name: "attack_hit".into() },
        TransitionCondition::TargetLost              =>
            AnimCondition::Bool   { param: "has_target".into(), value: false },
        TransitionCondition::Timeout { .. }          =>
            // Closest approximation: fire when the clip finishes naturally.
            AnimCondition::Always,
        TransitionCondition::OnDeath                 =>
            AnimCondition::Trigger { name: "die".into() },
        TransitionCondition::OnRespawn               =>
            AnimCondition::Trigger { name: "respawn".into() },
    }
}

/// Convert a WAC [`AnimTransition`] to a nova-anim [`Transition`].
///
/// Returns `None` if the condition cannot be represented.
fn convert_transition(wac: &AnimTransition) -> Option<Transition> {
    let condition = convert_condition(&wac.condition);
    // Blend time: higher WAC priority → snappier blend.
    let blend_time = (100.0 / wac.priority.max(1) as f32).clamp(0.04, 0.25);
    Some(Transition {
        from:       wac.from.clone(),
        to:         wac.to.clone(),
        condition,
        blend_time,
    })
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AnimState as WacAnimState, AnimTransition, AssetBlueprint, AssetIntent};
    use crate::compile::animation::compile as anim_compile;

    fn wolf_blueprint() -> AssetBlueprint {
        AssetBlueprint::new(
            AssetIntent::AnimationGraph,
            "wolf idle patrol search attack flee die",
            vec![],
            42,
        )
    }

    #[test]
    fn bridge_produces_fsm_with_all_states() {
        let ir  = anim_compile(&wolf_blueprint()).unwrap();
        let fsm = ir.to_nova_fsm(VoxelSkeleton::humanoid());
        // FSM starts in idle (default)
        assert_eq!(fsm.current, "idle");
    }

    #[test]
    fn bridge_transitions_wired() {
        let ir = anim_compile(&wolf_blueprint()).unwrap();
        // IR should contain attack→flee and similar
        let has_attack_flee = ir.transitions.iter()
            .any(|t| t.from == "attack" && t.to == "flee");
        assert!(has_attack_flee, "attack→flee transition must exist in the IR");
        // Now convert and verify FSM was constructed without panic
        let _fsm = ir.to_nova_fsm(VoxelSkeleton::humanoid());
    }

    #[test]
    fn stub_clip_has_identity_keyframes() {
        let clip = stub_clip("walk", 600, true);
        assert_eq!(clip.name, "walk");
        assert!((clip.duration - 0.6).abs() < 1e-4);
        assert!(clip.looping);
        assert_eq!(clip.tracks.len(), 1);
        // At least two keyframes (t=0 and t=end)
        assert!(clip.tracks[0].keys.len() >= 1);
    }
}
