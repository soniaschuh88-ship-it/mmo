//! Chunk coordinate types.
//!
//! A chunk is a 64×64×64 voxel region of the world. The chunk grid uses
//! signed 32-bit coordinates, giving a world extent of ±137 billion voxels
//! per axis at LOD 0.

use serde::{Deserialize, Serialize};

/// Position of a chunk in the world chunk grid.
///
/// Chunk (x, y, z) covers world voxels
/// `[x*64 .. x*64+63, y*64 .. y*64+63, z*64 .. z*64+63]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkCoord {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Canonical 12-byte LE representation.
    pub fn to_bytes(self) -> [u8; 12] {
        let mut buf = [0u8; 12];
        buf[0..4].copy_from_slice(&self.x.to_le_bytes());
        buf[4..8].copy_from_slice(&self.y.to_le_bytes());
        buf[8..12].copy_from_slice(&self.z.to_le_bytes());
        buf
    }
}

impl std::fmt::Display for ChunkCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "chunk({},{},{})", self.x, self.y, self.z)
    }
}

/// Unique identifier for a chunk, combining spatial position and LOD level.
///
/// LOD 0 = full-resolution 64³.  LOD k = 64/2ᵏ effective voxel resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct ChunkId {
    pub coord: ChunkCoord,
    /// Level-of-detail tier (0 = full simulation).
    pub lod: u8,
}

impl ChunkId {
    pub fn new(x: i32, y: i32, z: i32, lod: u8) -> Self {
        Self { coord: ChunkCoord::new(x, y, z), lod }
    }

    /// Full-resolution chunk at the given coordinate.
    pub fn full(coord: ChunkCoord) -> Self {
        Self { coord, lod: 0 }
    }

    /// Canonical 13-byte LE representation for hashing.
    pub fn to_bytes(self) -> [u8; 13] {
        let mut buf = [0u8; 13];
        buf[0..12].copy_from_slice(&self.coord.to_bytes());
        buf[12] = self.lod;
        buf
    }
}

impl std::fmt::Display for ChunkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "chunk({},{},{},lod={})", self.coord.x, self.coord.y, self.coord.z, self.lod)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_id_ordering() {
        // BTreeMap ordering must be stable and deterministic
        let a = ChunkId::new(0, 0, 0, 0);
        let b = ChunkId::new(1, 0, 0, 0);
        let c = ChunkId::new(0, 0, 0, 1);
        assert!(a < b);
        assert!(a < c);
    }

    #[test]
    fn to_bytes_round_trip() {
        let id = ChunkId::new(-1, 2, 300, 1);
        let b = id.to_bytes();
        assert_eq!(b[12], 1_u8); // lod
        let x = i32::from_le_bytes(b[0..4].try_into().unwrap());
        assert_eq!(x, -1);
    }

    #[test]
    fn display() {
        let id = ChunkId::full(ChunkCoord::new(3, 4, 5));
        assert_eq!(id.to_string(), "chunk(3,4,5,lod=0)");
    }
}
