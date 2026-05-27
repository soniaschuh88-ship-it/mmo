//! AnimStateMachine — FSM that drives a [`VoxelSkeleton`] from [`AnimClip`]s.
//!
//! ## Standard states
//!
//! | State    | Clip       | Loop | Trigger |
//! |----------|------------|------|---------|
//! | `idle`   | breathing  | ✓    | —       |
//! | `walk`   | leg swing  | ✓    | `is_moving = true` |
//! | `attack` | arm swing  | ✗    | trigger `attack` |
//! | `hurt`   | recoil     | ✗    | trigger `hurt` |
//! | `die`    | collapse   | ✗    | trigger `die` |
//!
//! ## Integration
//!
//! The Synthesis AI faction (bifrost-synthesis) drives entity FSMs through
//! the same bool params + triggers that a human player uses — maintaining the
//! symmetry guarantee from the project architecture.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use nova_core::transform::{Quat, Vec3};

use crate::clip::{AnimClip, Keyframe, Track};
use crate::skeleton::{BonePose, VoxelSkeleton};

// ─── Condition ────────────────────────────────────────────────────────────────

/// A predicate that can fire a transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnimCondition {
    /// Fire when the current clip finishes (only meaningful for non-looping clips).
    Always,
    /// Fire when a named bool param equals `value`.
    Bool   { param: String, value: bool },
    /// Fire when a float param exceeds `threshold`.
    FloatGt { param: String, threshold: f32 },
    /// Fire when a float param is below `threshold`.
    FloatLt { param: String, threshold: f32 },
    /// Fire once when a named trigger is set; consumes the trigger.
    Trigger { name: String },
}

// ─── Transition ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    /// Source state (`"*"` matches any state).
    pub from:       String,
    pub to:         String,
    pub condition:  AnimCondition,
    /// Cross-fade duration in seconds.
    pub blend_time: f32,
}

// ─── AnimState ────────────────────────────────────────────────────────────────

pub struct AnimState {
    pub name: String,
    pub clip: AnimClip,
}

// ─── Params ───────────────────────────────────────────────────────────────────

/// Runtime parameters that drive transitions.
#[derive(Debug, Clone, Default)]
pub struct AnimParams {
    pub bools:    BTreeMap<String, bool>,
    pub floats:   BTreeMap<String, f32>,
    pub triggers: BTreeMap<String, bool>,
}

impl AnimParams {
    pub fn set_bool(&mut self, k: &str, v: bool)  { self.bools.insert(k.to_string(), v); }
    pub fn set_float(&mut self, k: &str, v: f32)  { self.floats.insert(k.to_string(), v); }
    pub fn set_trigger(&mut self, k: &str)        { self.triggers.insert(k.to_string(), true); }
    pub fn consume_trigger(&mut self, k: &str) -> bool {
        self.triggers.remove(k).unwrap_or(false)
    }
}

// ─── AnimStateMachine ─────────────────────────────────────────────────────────

/// Full animation FSM for one character entity.
pub struct AnimStateMachine {
    states:       BTreeMap<String, AnimState>,
    transitions:  Vec<Transition>,

    /// Name of the currently active state.
    pub current:  String,
    prev:          String,

    /// Playback time (seconds) within the active state.
    pub time:     f32,
    /// Cross-fade progress [0→1] between `prev` and `current`.
    blend_t:      f32,
    /// `1 / blend_duration` — how fast blend_t advances per second.
    blend_spd:    f32,

    pub params:   AnimParams,
    pub skeleton: VoxelSkeleton,
}

impl AnimStateMachine {
    pub fn new(skeleton: VoxelSkeleton, default_state: impl Into<String>) -> Self {
        let s = default_state.into();
        Self {
            states:      BTreeMap::new(),
            transitions: vec![],
            current:     s.clone(),
            prev:        s,
            time:        0.0,
            blend_t:     1.0,
            blend_spd:   0.0,
            params:      AnimParams::default(),
            skeleton,
        }
    }

    pub fn add_state(&mut self, s: AnimState) {
        self.states.insert(s.name.clone(), s);
    }

    pub fn add_transition(&mut self, t: Transition) {
        self.transitions.push(t);
    }

    // ── Update ────────────────────────────────────────────────────────────────

    /// Advance the FSM by `dt` seconds.  Evaluates transitions, advances
    /// playback, blends skeleton poses.
    pub fn update(&mut self, dt: f32) {
        self.time   += dt;
        self.blend_t = (self.blend_t + dt * self.blend_spd).min(1.0);

        // Collect matching transitions (borrow-checker friendly)
        let matching: Vec<Transition> = self.transitions.iter()
            .filter(|t| t.from == self.current || t.from == "*")
            .cloned()
            .collect();

        for tr in &matching {
            if self.eval_condition(&tr.condition) {
                self.go(&tr.to, tr.blend_time);
                break;
            }
        }

        self.apply_pose_to_skeleton();
    }

    fn eval_condition(&mut self, cond: &AnimCondition) -> bool {
        match cond {
            AnimCondition::Always => {
                self.states.get(&self.current)
                    .map(|s| !s.clip.looping && self.time >= s.clip.duration)
                    .unwrap_or(false)
            }
            AnimCondition::Bool { param, value } => {
                self.params.bools.get(param.as_str()).copied().unwrap_or(false) == *value
            }
            AnimCondition::FloatGt { param, threshold } => {
                self.params.floats.get(param.as_str()).copied().unwrap_or(0.0) > *threshold
            }
            AnimCondition::FloatLt { param, threshold } => {
                self.params.floats.get(param.as_str()).copied().unwrap_or(0.0) < *threshold
            }
            AnimCondition::Trigger { name } => self.params.consume_trigger(name),
        }
    }

    fn go(&mut self, target: &str, blend_time: f32) {
        if self.current == target { return; }
        self.prev    = self.current.clone();
        self.current = target.to_string();
        self.time    = 0.0;
        if blend_time < 1e-4 {
            self.blend_t   = 1.0;
            self.blend_spd = 0.0;
        } else {
            self.blend_t   = 0.0;
            self.blend_spd = 1.0 / blend_time;
        }
    }

    fn apply_pose_to_skeleton(&mut self) {
        let t = self.time;
        let bone_names: Vec<String> = self.skeleton.bones.iter()
            .map(|b| b.name.clone())
            .collect();

        for bone in &bone_names {
            let cur  = self.states.get(&self.current).and_then(|s| s.clip.sample_bone(bone, t));
            let prev = if self.blend_t < 1.0 {
                self.states.get(&self.prev).and_then(|s| s.clip.sample_bone(bone, t))
            } else {
                None
            };

            let pose = match (cur, prev) {
                (Some(c), Some(p)) => {
                    let bt = self.blend_t;
                    BonePose {
                        translation: p.position.lerp(c.position, bt),
                        rotation:    p.rotation.slerp(c.rotation, bt),
                        scale:       p.scale.lerp(c.scale, bt),
                    }
                }
                (Some(c), None) => BonePose {
                    translation: c.position,
                    rotation:    c.rotation,
                    scale:       c.scale,
                },
                _ => BonePose::identity(),
            };

            self.skeleton.set_pose(bone, pose);
        }
    }

    // ── Shortcut setters ─────────────────────────────────────────────────────

    /// Set `is_moving` bool — drives idle ↔ walk transition.
    pub fn set_moving(&mut self, v: bool)   { self.params.set_bool("is_moving", v); }
    /// One-shot attack trigger.
    pub fn trigger_attack(&mut self)        { self.params.set_trigger("attack"); }
    /// One-shot hurt trigger.
    pub fn trigger_hurt(&mut self)          { self.params.set_trigger("hurt"); }
    /// One-shot die trigger — state does not loop back.
    pub fn trigger_die(&mut self)           { self.params.set_trigger("die"); }
}

// ─── Standard character FSM ───────────────────────────────────────────────────

/// Build the standard idle·walk·attack·hurt·die FSM for any humanoid entity.
///
/// Used by player characters, NPCs, and Synthesis AI agents alike.
pub fn standard_character_fsm(skeleton: VoxelSkeleton) -> AnimStateMachine {
    let mut fsm = AnimStateMachine::new(skeleton, "idle");

    fsm.add_state(AnimState { name: "idle".into(),   clip: mk_idle()   });
    fsm.add_state(AnimState { name: "walk".into(),   clip: mk_walk()   });
    fsm.add_state(AnimState { name: "attack".into(), clip: mk_attack() });
    fsm.add_state(AnimState { name: "hurt".into(),   clip: mk_hurt()   });
    fsm.add_state(AnimState { name: "die".into(),    clip: mk_die()    });

    use AnimCondition::*;
    // Walk ↔ idle
    fsm.add_transition(Transition { from:"idle".into(),   to:"walk".into(),
        condition: Bool { param:"is_moving".into(), value:true  }, blend_time: 0.12 });
    fsm.add_transition(Transition { from:"walk".into(),   to:"idle".into(),
        condition: Bool { param:"is_moving".into(), value:false }, blend_time: 0.12 });
    // Attack (from any state, returns to idle on clip end)
    fsm.add_transition(Transition { from:"*".into(),      to:"attack".into(),
        condition: Trigger { name:"attack".into() }, blend_time: 0.05 });
    fsm.add_transition(Transition { from:"attack".into(), to:"idle".into(),
        condition: Always, blend_time: 0.10 });
    // Hurt (from any state, returns to idle)
    fsm.add_transition(Transition { from:"*".into(),      to:"hurt".into(),
        condition: Trigger { name:"hurt".into() }, blend_time: 0.04 });
    fsm.add_transition(Transition { from:"hurt".into(),   to:"idle".into(),
        condition: Always, blend_time: 0.10 });
    // Die (terminal — no outgoing transition)
    fsm.add_transition(Transition { from:"*".into(),      to:"die".into(),
        condition: Trigger { name:"die".into() }, blend_time: 0.05 });

    fsm
}

// ─── Procedural clip builders ─────────────────────────────────────────────────

fn mk_idle() -> AnimClip {
    let mut c = AnimClip::new("idle", 2.0, true);
    let mut h = Track::new("head");
    h.keys.push(Keyframe::identity(0.0));
    h.keys.push(Keyframe { time: 1.0, position: Vec3::new(0.0, 0.06, 0.0), ..Keyframe::identity(1.0) });
    h.keys.push(Keyframe::identity(2.0));
    c.tracks.push(h);
    c
}

fn mk_walk() -> AnimClip {
    let mut c = AnimClip::new("walk", 0.5, true);
    let sy = 0.08_f32;

    let leg_keys_a = |phase: f32| {
        vec![
            Keyframe { time:0.00, position:Vec3::new(0.0,-sy,0.0), rotation:Quat::from_euler( 0.3*phase,0.0,0.0), ..Keyframe::identity(0.00) },
            Keyframe { time:0.25, position:Vec3::new(0.0, sy,0.0), rotation:Quat::from_euler(-0.3*phase,0.0,0.0), ..Keyframe::identity(0.25) },
            Keyframe { time:0.50, position:Vec3::new(0.0,-sy,0.0), rotation:Quat::from_euler( 0.3*phase,0.0,0.0), ..Keyframe::identity(0.50) },
        ]
    };
    let arm_keys_a = |phase: f32| {
        vec![
            Keyframe { time:0.00, rotation:Quat::from_euler(-0.3*phase,0.0,0.0), ..Keyframe::identity(0.00) },
            Keyframe { time:0.25, rotation:Quat::from_euler( 0.3*phase,0.0,0.0), ..Keyframe::identity(0.25) },
            Keyframe { time:0.50, rotation:Quat::from_euler(-0.3*phase,0.0,0.0), ..Keyframe::identity(0.50) },
        ]
    };

    let mut ll = Track::new("leg_l"); ll.keys = leg_keys_a( 1.0);
    let mut lr = Track::new("leg_r"); lr.keys = leg_keys_a(-1.0);
    let mut al = Track::new("arm_l"); al.keys = arm_keys_a(-1.0);
    let mut ar = Track::new("arm_r"); ar.keys = arm_keys_a( 1.0);
    c.tracks.extend([ll, lr, al, ar]);
    c
}

fn mk_attack() -> AnimClip {
    let mut c = AnimClip::new("attack", 0.40, false);
    let mut ar = Track::new("arm_r");
    ar.keys.push(Keyframe { time:0.00, rotation:Quat::from_euler(-0.5,0.0,0.0), ..Keyframe::identity(0.00) });
    ar.keys.push(Keyframe { time:0.15, rotation:Quat::from_euler( 1.2,0.0,0.0), ..Keyframe::identity(0.15) });
    ar.keys.push(Keyframe::identity(0.40));
    c.tracks.push(ar);
    c
}

fn mk_hurt() -> AnimClip {
    let mut c = AnimClip::new("hurt", 0.25, false);
    let mut ub = Track::new("upper_body");
    ub.keys.push(Keyframe::identity(0.00));
    ub.keys.push(Keyframe { time:0.08, rotation:Quat::from_euler(-0.4,0.0,0.0), ..Keyframe::identity(0.08) });
    ub.keys.push(Keyframe::identity(0.25));
    c.tracks.push(ub);
    c
}

fn mk_die() -> AnimClip {
    let mut c = AnimClip::new("die", 0.80, false);
    let mut root = Track::new("root");
    root.keys.push(Keyframe::identity(0.00));
    root.keys.push(Keyframe {
        time:     0.80,
        position: Vec3::new(0.0, -6.0, 0.0),
        rotation: Quat::from_euler(1.5708, 0.0, 0.0), // 90°
        scale:    Vec3::ONE,
    });
    c.tracks.push(root);
    c
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::VoxelSkeleton;

    fn fsm() -> AnimStateMachine { standard_character_fsm(VoxelSkeleton::humanoid()) }

    #[test]
    fn starts_in_idle() {
        assert_eq!(fsm().current, "idle");
    }

    #[test]
    fn idle_to_walk_on_is_moving() {
        let mut f = fsm();
        f.set_moving(true);
        f.update(0.016);
        assert_eq!(f.current, "walk");
    }

    #[test]
    fn walk_back_to_idle() {
        let mut f = fsm();
        f.set_moving(true);
        f.update(0.016);
        f.set_moving(false);
        f.update(0.016);
        assert_eq!(f.current, "idle");
    }

    #[test]
    fn attack_trigger_fires() {
        let mut f = fsm();
        f.trigger_attack();
        f.update(0.016);
        assert_eq!(f.current, "attack");
    }

    #[test]
    fn attack_returns_to_idle_after_clip() {
        let mut f = fsm();
        f.trigger_attack();
        f.update(0.016);
        // advance past clip duration (0.40s) + blend (0.10s)
        for _ in 0..40 { f.update(0.016); }
        assert_eq!(f.current, "idle");
    }

    #[test]
    fn hurt_then_idle() {
        let mut f = fsm();
        f.trigger_hurt();
        f.update(0.016);
        assert_eq!(f.current, "hurt");
        for _ in 0..25 { f.update(0.016); }
        assert_eq!(f.current, "idle");
    }

    #[test]
    fn die_is_terminal() {
        let mut f = fsm();
        f.trigger_die();
        f.update(0.016);
        assert_eq!(f.current, "die");
        // advance far past clip — should stay in die
        for _ in 0..120 { f.update(0.016); }
        assert_eq!(f.current, "die");
    }

    #[test]
    fn trigger_consumed_after_use() {
        let mut f = fsm();
        f.trigger_attack();
        f.update(0.016); // transition fires + consumes trigger
        assert_eq!(f.current, "attack");
        // trigger should not fire again
        let consumed = f.params.consume_trigger("attack");
        assert!(!consumed);
    }
}
