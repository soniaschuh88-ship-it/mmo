//! Vec3 — deterministic f64 3D vector.
//!
//! # Determinism contract
//!
//! All operations use IEEE 754 double-precision arithmetic. Determinism
//! holds as long as:
//! - All peers compile with the same floating-point mode
//! - No fused-multiply-add (FMA) instructions are used
//! - WASM's deterministic float semantics are relied upon for cross-platform equality
//!
//! For voxel distances and radii, we compare integer-squared distances
//! (avoiding sqrt entirely) wherever possible to eliminate FP divergence.

use serde::{Deserialize, Serialize};

/// A 3D vector with `f64` components.
///
/// Used for voxel velocities and physics forces. Positions use `(i32, i32, i32)` keys.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    #[inline]
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn length_sq(self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// True if all components are exactly zero.
    #[inline]
    pub fn is_zero(self) -> bool {
        self.x == 0.0 && self.y == 0.0 && self.z == 0.0
    }

    /// Integer squared distance between two voxel centers.
    ///
    /// Used for radius checks to avoid floating-point sqrt.
    #[inline]
    pub fn int_dist_sq(ax: i32, ay: i32, az: i32, bx: i32, by: i32, bz: i32) -> i64 {
        let dx = (ax - bx) as i64;
        let dy = (ay - by) as i64;
        let dz = (az - bz) as i64;
        dx * dx + dy * dy + dz * dz
    }

    /// Canonical 24-byte representation: IEEE 754 bits in little-endian order.
    ///
    /// Used for BLAKE3 state hashing. NaN/infinity are valid encodings of
    /// unusual states and are included as-is.
    pub fn canonical_bytes(self) -> [u8; 24] {
        let mut buf = [0u8; 24];
        buf[0..8].copy_from_slice(&self.x.to_bits().to_le_bytes());
        buf[8..16].copy_from_slice(&self.y.to_bits().to_le_bytes());
        buf[16..24].copy_from_slice(&self.z.to_bits().to_le_bytes());
        buf
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, s: f64) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_dist_sq_correct() {
        // 3-4-5 right triangle: dist_sq = 25
        assert_eq!(Vec3::int_dist_sq(0, 0, 0, 3, 4, 0), 25);
    }

    #[test]
    fn canonical_bytes_deterministic() {
        let v = Vec3::new(1.5, -2.0, 0.0);
        assert_eq!(v.canonical_bytes(), v.canonical_bytes());
    }

    #[test]
    fn zero_detection() {
        assert!(Vec3::ZERO.is_zero());
        assert!(!Vec3::new(0.0, 0.0, 1.0).is_zero());
    }
}
