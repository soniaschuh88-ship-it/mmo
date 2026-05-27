//! Linear algebra primitives — [`Vec3`], [`Quat`], [`Mat4`], [`Transform3D`].
//!
//! All types use `f32` for WebGPU compatibility.  [`Mat4`] is **column-major**,
//! matching the WGSL / wgpu memory layout.

use serde::{Deserialize, Serialize};

// ─── Vec3 ─────────────────────────────────────────────────────────────────────

/// 3-component f32 vector used throughout the engine.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO:    Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE:     Self = Self { x: 1.0, y: 1.0, z: 1.0 };
    pub const UP:      Self = Self { x: 0.0, y: 1.0, z: 0.0 };
    /// Canonical camera look-at direction (–Z in right-handed world space).
    pub const FORWARD: Self = Self { x: 0.0, y: 0.0, z: -1.0 };
    pub const RIGHT:   Self = Self { x: 1.0, y: 0.0, z: 0.0 };

    #[inline] pub fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z } }
    #[inline] pub fn splat(v: f32) -> Self { Self { x: v, y: v, z: v } }

    #[inline] pub fn length_sq(self) -> f32 { self.x*self.x + self.y*self.y + self.z*self.z }
    #[inline] pub fn length(self)    -> f32 { self.length_sq().sqrt() }

    pub fn normalized(self) -> Self {
        let l = self.length();
        if l < 1e-8 { Self::ZERO } else { Self::new(self.x/l, self.y/l, self.z/l) }
    }

    #[inline] pub fn dot(self, r: Self) -> f32 { self.x*r.x + self.y*r.y + self.z*r.z }

    pub fn cross(self, r: Self) -> Self {
        Self::new(
            self.y * r.z - self.z * r.y,
            self.z * r.x - self.x * r.z,
            self.x * r.y - self.y * r.x,
        )
    }

    pub fn lerp(self, o: Self, t: f32) -> Self {
        Self::new(
            self.x + (o.x - self.x) * t,
            self.y + (o.y - self.y) * t,
            self.z + (o.z - self.z) * t,
        )
    }

    #[inline] pub fn distance(self, o: Self) -> f32 { (self - o).length() }
}

impl std::ops::Add for Vec3 { type Output=Self; fn add(self,r:Self)->Self{Self::new(self.x+r.x,self.y+r.y,self.z+r.z)} }
impl std::ops::Sub for Vec3 { type Output=Self; fn sub(self,r:Self)->Self{Self::new(self.x-r.x,self.y-r.y,self.z-r.z)} }
impl std::ops::Mul<f32> for Vec3 { type Output=Self; fn mul(self,s:f32)->Self{Self::new(self.x*s,self.y*s,self.z*s)} }
impl std::ops::Neg for Vec3 { type Output=Self; fn neg(self)->Self{Self::new(-self.x,-self.y,-self.z)} }
impl Default for Vec3 { fn default() -> Self { Self::ZERO } }

// ─── Quat ─────────────────────────────────────────────────────────────────────

/// Unit quaternion for 3-D rotation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub const IDENTITY: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    /// Rotation of `angle_rad` around `axis` (need not be unit length).
    pub fn from_axis_angle(axis: Vec3, angle_rad: f32) -> Self {
        let a = axis.normalized();
        let s = (angle_rad * 0.5).sin();
        Self { x: a.x*s, y: a.y*s, z: a.z*s, w: (angle_rad * 0.5).cos() }
    }

    /// Euler angles in radians (pitch X, yaw Y, roll Z) — ZYX convention.
    pub fn from_euler(pitch: f32, yaw: f32, roll: f32) -> Self {
        let (sp,cp) = ((pitch*0.5).sin(), (pitch*0.5).cos());
        let (sy,cy) = ((yaw  *0.5).sin(), (yaw  *0.5).cos());
        let (sr,cr) = ((roll *0.5).sin(), (roll *0.5).cos());
        Self {
            x: sp*cy*cr + cp*sy*sr,
            y: cp*sy*cr - sp*cy*sr,
            z: cp*cy*sr + sp*sy*cr,
            w: cp*cy*cr - sp*sy*sr,
        }
    }

    pub fn normalized(self) -> Self {
        let l = (self.x*self.x + self.y*self.y + self.z*self.z + self.w*self.w).sqrt();
        if l < 1e-8 { Self::IDENTITY } else {
            Self { x: self.x/l, y: self.y/l, z: self.z/l, w: self.w/l }
        }
    }

    /// Spherical linear interpolation.
    pub fn slerp(self, mut o: Self, t: f32) -> Self {
        let mut d = self.x*o.x + self.y*o.y + self.z*o.z + self.w*o.w;
        if d < 0.0 { o = Quat { x:-o.x, y:-o.y, z:-o.z, w:-o.w }; d = -d; }
        let d = d.clamp(-1.0, 1.0);
        if d > 0.9995 {
            return Quat {
                x: self.x + (o.x-self.x)*t, y: self.y + (o.y-self.y)*t,
                z: self.z + (o.z-self.z)*t, w: self.w + (o.w-self.w)*t,
            }.normalized();
        }
        let th0 = d.acos();
        let th  = th0 * t;
        let s0  = (th0 - th).sin() / th0.sin();
        let s1  = th.sin() / th0.sin();
        Quat { x: self.x*s0+o.x*s1, y: self.y*s0+o.y*s1, z: self.z*s0+o.z*s1, w: self.w*s0+o.w*s1 }
    }

    /// Rotate a vector by this quaternion.
    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        let q = self;
        let uv  = Vec3::new(q.y*v.z-q.z*v.y, q.z*v.x-q.x*v.z, q.x*v.y-q.y*v.x);
        let uuv = Vec3::new(q.y*uv.z-q.z*uv.y, q.z*uv.x-q.x*uv.z, q.x*uv.y-q.y*uv.x);
        v + uv*(2.0*q.w) + uuv*2.0
    }
}

impl Default for Quat { fn default() -> Self { Self::IDENTITY } }

// ─── Mat4 ─────────────────────────────────────────────────────────────────────

/// Column-major 4×4 matrix for GPU upload.
///
/// Index layout: `self.0[col][row]` — matches WGSL `mat4x4<f32>` / wgpu memory.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Mat4(pub [[f32; 4]; 4]);

impl Mat4 {
    pub const IDENTITY: Self = Self([
        [1.0,0.0,0.0,0.0],
        [0.0,1.0,0.0,0.0],
        [0.0,0.0,1.0,0.0],
        [0.0,0.0,0.0,1.0],
    ]);

    /// Perspective projection (right-handed, reversed-Z friendly).
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f  = 1.0 / (fov_y * 0.5).tan();
        let nf = 1.0 / (near - far);
        Self([
            [f/aspect, 0.0, 0.0,              0.0],
            [0.0,      f,   0.0,              0.0],
            [0.0,      0.0, (far+near)*nf,   -1.0],
            [0.0,      0.0, 2.0*far*near*nf,  0.0],
        ])
    }

    /// View matrix from eye position, look-at center, and up vector.
    pub fn look_at(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        let f = (center - eye).normalized();
        let s = f.cross(up).normalized();
        let u = s.cross(f);
        Self([
            [s.x,           u.x,           -f.x,           0.0],
            [s.y,           u.y,           -f.y,           0.0],
            [s.z,           u.z,           -f.z,           0.0],
            [-eye.dot(s),  -eye.dot(u),    eye.dot(f),     1.0],
        ])
    }

    /// TRS decomposition → matrix (translate × rotate × scale).
    pub fn from_trs(t: Vec3, r: Quat, s: Vec3) -> Self {
        let (rx2,ry2,rz2) = (r.x*r.x, r.y*r.y, r.z*r.z);
        let (rxy,rxz,ryz) = (r.x*r.y, r.x*r.z, r.y*r.z);
        let (rwx,rwy,rwz) = (r.w*r.x, r.w*r.y, r.w*r.z);
        Self([
            [s.x*(1.0-2.0*(ry2+rz2)), s.x*2.0*(rxy+rwz),       s.x*2.0*(rxz-rwy),       0.0],
            [s.y*2.0*(rxy-rwz),        s.y*(1.0-2.0*(rx2+rz2)), s.y*2.0*(ryz+rwx),       0.0],
            [s.z*2.0*(rxz+rwy),        s.z*2.0*(ryz-rwx),       s.z*(1.0-2.0*(rx2+ry2)), 0.0],
            [t.x,                      t.y,                      t.z,                      1.0],
        ])
    }

    /// Matrix multiplication: `a * b`.
    pub fn mul(a: &Self, b: &Self) -> Self {
        let mut o = [[0f32; 4]; 4];
        for c in 0..4 { for r in 0..4 { for k in 0..4 { o[c][r] += a.0[k][r] * b.0[c][k]; } } }
        Self(o)
    }

    /// Flat `[f32; 16]` in column-major order — ready for a `wgpu` uniform buffer.
    pub fn as_f32_array(&self) -> [f32; 16] {
        let mut a = [0f32; 16];
        for c in 0..4 { for r in 0..4 { a[c*4+r] = self.0[c][r]; } }
        a
    }
}

impl Default for Mat4 { fn default() -> Self { Self::IDENTITY } }

// ─── Transform3D ──────────────────────────────────────────────────────────────

/// World-space transform: position, orientation, scale.
///
/// This is the primary component attached to every visible entity.  The
/// renderer reads `to_matrix()` to obtain the model matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform3D {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale:    Vec3,
}

impl Transform3D {
    pub fn new() -> Self {
        Self { position: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE }
    }

    /// Positioned at `position`, identity rotation, unit scale.
    pub fn at(position: Vec3) -> Self { Self { position, ..Self::new() } }

    /// TRS model matrix for GPU upload.
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_trs(self.position, self.rotation, self.scale)
    }

    pub fn forward(&self) -> Vec3 { self.rotation.rotate_vec3(Vec3::FORWARD) }
    pub fn right(&self)   -> Vec3 { self.rotation.rotate_vec3(Vec3::RIGHT) }
    pub fn up(&self)      -> Vec3 { self.rotation.rotate_vec3(Vec3::UP) }
}

impl Default for Transform3D { fn default() -> Self { Self::new() } }

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec3_cross_right_forward_gives_up() {
        // RIGHT(1,0,0) × FORWARD(0,0,-1) = UP(0,1,0)
        let got = Vec3::RIGHT.cross(Vec3::FORWARD);
        assert!((got.x).abs() < 1e-5);
        assert!((got.y - 1.0).abs() < 1e-5);
        assert!((got.z).abs() < 1e-5);
    }

    #[test]
    fn vec3_normalize() {
        let v = Vec3::new(3.0, 0.0, 4.0).normalized();
        assert!((v.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn quat_identity_preserves_vector() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let r = Quat::IDENTITY.rotate_vec3(v);
        assert!((r.x - v.x).abs() < 1e-5);
        assert!((r.y - v.y).abs() < 1e-5);
        assert!((r.z - v.z).abs() < 1e-5);
    }

    #[test]
    fn mat4_perspective_is_finite() {
        let m = Mat4::perspective(std::f32::consts::FRAC_PI_4, 16.0/9.0, 0.1, 1000.0);
        assert!(m.0[0][0].is_finite() && m.0[2][2].is_finite());
    }

    #[test]
    fn mat4_identity_mul() {
        let a = Mat4::IDENTITY;
        let b = Mat4::from_trs(Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
        let r = Mat4::mul(&a, &b);
        assert!((r.0[3][0] - 5.0).abs() < 1e-5);
    }

    #[test]
    fn transform_translation_in_matrix() {
        let t = Transform3D::at(Vec3::new(7.0, 0.0, 0.0));
        let m = t.to_matrix();
        // col 3, row 0 = tx
        assert!((m.0[3][0] - 7.0).abs() < 1e-5);
    }

    #[test]
    fn quat_slerp_halfway() {
        let q0 = Quat::IDENTITY;
        let q1 = Quat::from_axis_angle(Vec3::UP, std::f32::consts::FRAC_PI_2);
        let mid = q0.slerp(q1, 0.5);
        // |mid| should be ≈ 1.0
        let len = (mid.x*mid.x+mid.y*mid.y+mid.z*mid.z+mid.w*mid.w).sqrt();
        assert!((len - 1.0).abs() < 1e-5);
    }
}
