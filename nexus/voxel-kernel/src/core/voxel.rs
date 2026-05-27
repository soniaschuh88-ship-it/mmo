//! Voxel — the atomic unit of world state.
//!
//! Packed to 4 bytes for cache-efficient chunk storage.
//!
//! # Layout
//!
//! ```text
//! [material: u16][flags: u8][light: u8]
//! ```
//!
//! - `material` — palette ID (0 = air)
//! - `flags`    — bitmask of `VoxelFlags`
//! - `light`    — packed 4+4 bits: high nibble = emission, low nibble = received light

use serde::{Deserialize, Serialize};

/// Voxel state flags.
pub mod flags {
    /// Occupies space, blocks movement.
    pub const SOLID:       u8 = 0b0000_0001;
    /// Emits light (requires emission level in `light` high nibble).
    pub const EMISSIVE:    u8 = 0b0000_0010;
    /// Fluid — flows and fills low areas.
    pub const LIQUID:      u8 = 0b0000_0100;
    /// Can be seen through (glass, ice, water).
    pub const TRANSPARENT: u8 = 0b0000_1000;
    /// AI-generated / procedural.
    pub const GENERATED:   u8 = 0b0001_0000;
    /// Modified since last chunk hash.
    pub const DIRTY:       u8 = 0b0010_0000;
}

/// A single voxel — 4 bytes packed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Voxel {
    /// Material palette ID. 0 = AIR (empty space).
    pub material: u16,
    /// Bitflags: solid, emissive, liquid, transparent, etc.
    pub flags:    u8,
    /// Packed light: high 4 bits = emission (0–15), low 4 bits = received (0–15).
    pub light:    u8,
}

impl Voxel {
    /// Empty space.
    pub const AIR: Self = Self { material: 0, flags: 0, light: 0 };

    /// Create a solid, opaque voxel with no emission.
    pub fn solid(material: u16) -> Self {
        Self { material, flags: flags::SOLID | flags::GENERATED, light: 0 }
    }

    /// Create a liquid voxel (water, lava).
    pub fn liquid(material: u16) -> Self {
        Self {
            material,
            flags: flags::LIQUID | flags::TRANSPARENT | flags::GENERATED,
            light: 0,
        }
    }

    /// Create an emissive voxel (glowstone, crystal).
    pub fn emissive(material: u16, emission_level: u8) -> Self {
        let light = (emission_level.min(15)) << 4;
        Self {
            material,
            flags: flags::SOLID | flags::EMISSIVE | flags::GENERATED,
            light,
        }
    }

    /// True if this voxel is air / empty.
    #[inline] pub fn is_air(self) -> bool  { self.material == 0 }
    /// True if this voxel blocks movement.
    #[inline] pub fn is_solid(self) -> bool { self.flags & flags::SOLID != 0 }
    /// True if this voxel emits light.
    #[inline] pub fn is_emissive(self) -> bool { self.flags & flags::EMISSIVE != 0 }
    /// True if this voxel allows light to pass through.
    #[inline] pub fn is_transparent(self) -> bool { self.flags & flags::TRANSPARENT != 0 }

    /// Light emission level (0–15).
    #[inline] pub fn emission(self) -> u8  { (self.light >> 4) & 0x0F }
    /// Received light level (0–15).
    #[inline] pub fn received_light(self) -> u8 { self.light & 0x0F }

    /// Canonical 4-byte representation for BLAKE3 hashing.
    #[inline]
    pub fn to_bytes(self) -> [u8; 4] {
        [
            (self.material & 0xFF) as u8,
            (self.material >> 8) as u8,
            self.flags,
            self.light,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn air_is_default() {
        let v = Voxel::default();
        assert!(v.is_air());
        assert!(!v.is_solid());
    }

    #[test]
    fn solid_flags() {
        let v = Voxel::solid(1);
        assert!(!v.is_air());
        assert!(v.is_solid());
        assert!(!v.is_emissive());
    }

    #[test]
    fn emission_packing() {
        let v = Voxel::emissive(23, 12); // material 23, level 12
        assert_eq!(v.emission(), 12);
        assert_eq!(v.material, 23);
        assert!(v.is_emissive());
    }

    #[test]
    fn bytes_roundtrip() {
        let v = Voxel::solid(300); // material ID > 255 (uses high byte)
        let b = v.to_bytes();
        let mat = u16::from(b[0]) | (u16::from(b[1]) << 8);
        assert_eq!(mat, 300);
    }
}
