//! VoxelCoord — integer world-space coordinate.

use serde::{Deserialize, Serialize};

/// A voxel position in world space.
///
/// Coordinates are signed 32-bit integers, giving a world extent of ±2 billion
/// voxels per axis — more than enough for any synthetic world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct VoxelCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl VoxelCoord {
    #[inline]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Canonical 12-byte little-endian representation for hashing.
    ///
    /// Field order is fixed: x, y, z. Never changes.
    #[inline]
    pub fn to_bytes(self) -> [u8; 12] {
        let mut buf = [0u8; 12];
        buf[0..4].copy_from_slice(&self.x.to_le_bytes());
        buf[4..8].copy_from_slice(&self.y.to_le_bytes());
        buf[8..12].copy_from_slice(&self.z.to_le_bytes());
        buf
    }
}

impl std::fmt::Display for VoxelCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{},{})", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_bytes() {
        let c = VoxelCoord::new(-100, 0, 32767);
        let b = c.to_bytes();
        let x = i32::from_le_bytes(b[0..4].try_into().unwrap());
        let y = i32::from_le_bytes(b[4..8].try_into().unwrap());
        let z = i32::from_le_bytes(b[8..12].try_into().unwrap());
        assert_eq!((x, y, z), (c.x, c.y, c.z));
    }

    #[test]
    fn display() {
        assert_eq!(VoxelCoord::new(1, 2, 3).to_string(), "(1,2,3)");
    }
}
