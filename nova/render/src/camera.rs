//! Camera3D — perspective or isometric-top camera with orbit support.
use nova_core::transform::{Mat4, Vec3};

// ─── CameraMode ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraMode {
    /// Standard 3-D perspective projection.
    Perspective,
    /// Top-down isometric view (oblique projection approximated via perspective).
    IsometricTop,
}

// ─── Camera3D ─────────────────────────────────────────────────────────────────

/// A game camera.  The host calls [`Camera3D::view_proj`] each frame and
/// uploads the result to the `Camera` uniform buffer.
#[derive(Debug, Clone)]
pub struct Camera3D {
    pub position: Vec3,
    pub target:   Vec3,
    pub up:       Vec3,
    /// Vertical field-of-view in radians (perspective mode).
    pub fov_y:    f32,
    pub aspect:   f32,
    pub near:     f32,
    pub far:      f32,
    pub mode:     CameraMode,
}

impl Camera3D {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Standard 3-D perspective camera.
    pub fn perspective(aspect: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 20.0, 30.0),
            target:   Vec3::ZERO,
            up:       Vec3::UP,
            fov_y:    std::f32::consts::FRAC_PI_4,
            aspect,
            near:     0.1,
            far:      2000.0,
            mode:     CameraMode::Perspective,
        }
    }

    /// Isometric top-down camera (used by `game.html` for the 2.5-D view).
    ///
    /// The angle is fixed at 30° pitch; yaw can be changed via [`Camera3D::orbit`].
    pub fn isometric(aspect: f32) -> Self {
        Self {
            position: Vec3::new(-20.0, 30.0, 20.0),
            target:   Vec3::ZERO,
            up:       Vec3::UP,
            fov_y:    0.4,   // slightly narrower FOV for isometric feel
            aspect,
            near:     0.1,
            far:      2000.0,
            mode:     CameraMode::IsometricTop,
        }
    }

    // ── Matrices ──────────────────────────────────────────────────────────────

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.position, self.target, self.up)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective(self.fov_y, self.aspect, self.near, self.far)
    }

    /// Combined view-projection matrix — upload to the `Camera` uniform buffer.
    pub fn view_proj(&self) -> Mat4 {
        Mat4::mul(&self.projection_matrix(), &self.view_matrix())
    }

    // ── Orbit ─────────────────────────────────────────────────────────────────

    /// Position the camera on a sphere of `dist` around `self.target`.
    ///
    /// - `yaw`   — horizontal angle in radians
    /// - `pitch` — vertical angle in radians (clamped to avoid gimbal flip)
    pub fn orbit(&mut self, yaw: f32, pitch: f32, dist: f32) {
        let pitch = pitch.clamp(-1.4, 1.4); // ±80°
        self.position = self.target + Vec3::new(
            dist * yaw.cos() * pitch.cos(),
            dist * pitch.sin(),
            dist * yaw.sin() * pitch.cos(),
        );
    }

    /// Move both position and target together (pan in world space).
    pub fn pan(&mut self, delta: Vec3) {
        self.position = self.position + delta;
        self.target   = self.target   + delta;
    }

    /// Aspect ratio helper — call after canvas resize.
    pub fn set_aspect(&mut self, w: f32, h: f32) {
        self.aspect = if h > 0.0 { w / h } else { 1.0 };
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_proj_is_finite() {
        let cam = Camera3D::perspective(16.0 / 9.0);
        let vp = cam.view_proj();
        // Every element should be a finite float
        for col in &vp.0 { for &v in col { assert!(v.is_finite(), "non-finite element: {v}"); } }
    }

    #[test]
    fn orbit_sets_position_at_correct_distance() {
        let mut cam = Camera3D::perspective(1.0);
        cam.target = Vec3::ZERO;
        cam.orbit(0.0, 0.0, 50.0);
        let actual_dist = cam.position.distance(cam.target);
        assert!((actual_dist - 50.0).abs() < 1e-3, "expected dist≈50, got {actual_dist}");
    }

    #[test]
    fn pan_moves_both_position_and_target() {
        let mut cam = Camera3D::perspective(1.0);
        let orig_pos    = cam.position;
        let orig_target = cam.target;
        let delta = Vec3::new(10.0, 0.0, 0.0);
        cam.pan(delta);
        assert!((cam.position.x - (orig_pos.x + 10.0)).abs() < 1e-5);
        assert!((cam.target.x   - (orig_target.x + 10.0)).abs() < 1e-5);
    }

    #[test]
    fn isometric_camera_constructed() {
        let cam = Camera3D::isometric(16.0 / 9.0);
        assert_eq!(cam.mode, CameraMode::IsometricTop);
        assert!(cam.position.y > 0.0); // above the world
    }
}
