//! Keyframe animation clips.
//!
//! A [`AnimClip`] is a collection of [`Track`]s — one per bone group.
//! Each track holds a sorted list of [`Keyframe`]s and supports
//! smooth interpolation between them.

use serde::{Deserialize, Serialize};

use nova_core::transform::{Quat, Vec3};

// ─── Keyframe ─────────────────────────────────────────────────────────────────

/// One keyframe: a point in time with a full local pose.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    /// Seconds from clip start.
    pub time:     f32,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale:    Vec3,
}

impl Keyframe {
    /// Identity pose at time `t`.
    pub fn identity(t: f32) -> Self {
        Self { time: t, position: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE }
    }
}

// ─── Track ────────────────────────────────────────────────────────────────────

/// Animation data for a single named bone group.
///
/// Keyframes must be stored in **ascending time** order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub bone_name: String,
    /// Keyframes, sorted by `time`.
    pub keys:      Vec<Keyframe>,
}

impl Track {
    pub fn new(bone_name: impl Into<String>) -> Self {
        Self { bone_name: bone_name.into(), keys: vec![] }
    }

    /// Sample the track at `t` seconds, interpolating between the two
    /// nearest keyframes.
    pub fn sample(&self, t: f32) -> Keyframe {
        match self.keys.len() {
            0 => return Keyframe::identity(t),
            1 => return self.keys[0].clone(),
            _ => {}
        }

        let idx = self.keys.partition_point(|k| k.time <= t);

        if idx == 0               { return self.keys[0].clone(); }
        if idx >= self.keys.len() { return self.keys.last().unwrap().clone(); }

        let k0 = &self.keys[idx - 1];
        let k1 = &self.keys[idx];
        let span = (k1.time - k0.time).max(f32::EPSILON);
        let blend = ((t - k0.time) / span).clamp(0.0, 1.0);

        Keyframe {
            time:     t,
            position: k0.position.lerp(k1.position, blend),
            rotation: k0.rotation.slerp(k1.rotation, blend),
            scale:    k0.scale.lerp(k1.scale, blend),
        }
    }
}

// ─── AnimClip ─────────────────────────────────────────────────────────────────

/// A complete animation clip — set of per-bone tracks with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimClip {
    pub name:     String,
    pub duration: f32,
    pub looping:  bool,
    pub tracks:   Vec<Track>,
}

impl AnimClip {
    pub fn new(name: impl Into<String>, duration: f32, looping: bool) -> Self {
        Self { name: name.into(), duration, looping, tracks: vec![] }
    }

    /// Sample a specific bone at `t` seconds.
    ///
    /// Returns `None` if no track exists for `bone`.
    pub fn sample_bone(&self, bone: &str, t: f32) -> Option<Keyframe> {
        let effective_t = if self.looping {
            t % self.duration.max(f32::EPSILON)
        } else {
            t.min(self.duration)
        };
        self.tracks.iter().find(|tr| tr.bone_name == bone).map(|tr| tr.sample(effective_t))
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_interpolates_position() {
        let mut tr = Track::new("leg_l");
        tr.keys.push(Keyframe::identity(0.0));
        tr.keys.push(Keyframe { time: 1.0, position: Vec3::new(0.0, 1.0, 0.0), ..Keyframe::identity(1.0) });
        let mid = tr.sample(0.5);
        assert!((mid.position.y - 0.5).abs() < 1e-5, "expected y≈0.5, got {}", mid.position.y);
    }

    #[test]
    fn track_clamps_before_start() {
        let mut tr = Track::new("head");
        tr.keys.push(Keyframe { time: 1.0, position: Vec3::new(0.0, 2.0, 0.0), ..Keyframe::identity(1.0) });
        let k = tr.sample(0.0);  // before first key
        assert!((k.position.y - 2.0).abs() < 1e-5);
    }

    #[test]
    fn clip_loops_correctly() {
        let mut clip = AnimClip::new("walk", 0.5, true);
        let mut tr = Track::new("head");
        tr.keys.push(Keyframe::identity(0.0));
        tr.keys.push(Keyframe { time: 0.5, position: Vec3::new(0.0, 0.2, 0.0), ..Keyframe::identity(0.5) });
        clip.tracks.push(tr);

        // t = 0.6 → loops to 0.1
        let k = clip.sample_bone("head", 0.6).unwrap();
        assert!(k.position.y < 0.1, "expected y < 0.1 but got {}", k.position.y);
    }

    #[test]
    fn clip_missing_bone_returns_none() {
        let clip = AnimClip::new("idle", 1.0, true);
        assert!(clip.sample_bone("nonexistent", 0.5).is_none());
    }
}
