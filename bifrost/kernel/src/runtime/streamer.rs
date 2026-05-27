//! ChunkStreamer — priority-ordered chunk request queue.
//!
//! The streamer sits between the WAC adapter (which requests chunks by biome)
//! and the WorldRuntime (which generates them). It deduplicates requests and
//! orders generation by distance from the viewer — closest chunks first.
//!
//! # Integration with BIFROST
//!
//! In the full MMO pipeline:
//! ```text
//! WAC → wac_adapter::apply_biome()
//!     → ChunkStreamer::request()
//!     → ChunkStreamer::drain() → VoxelChunk per batch
//!     → bifrost::stream(chunk)    [nexus-bifrost bridge]
//! ```

use std::collections::{BTreeMap, BTreeSet};

use crate::core::ChunkPos;

// ── ChunkRequest ──────────────────────────────────────────────────────────────

/// A single pending chunk generation request.
#[derive(Debug, Clone)]
pub struct ChunkRequest {
    pub pos:        ChunkPos,
    pub biome_name: String,
    /// Lower = higher priority (distance squared from viewer).
    pub priority:   u64,
}

// ── ChunkStreamer ─────────────────────────────────────────────────────────────

/// Priority-ordered chunk request queue.
///
/// Deduplicates: requesting the same `ChunkPos` twice is idempotent.
pub struct ChunkStreamer {
    /// Pending requests ordered by priority (ascending = closest first).
    /// Key = priority * 10^9 + deterministic hash of pos to break ties.
    pending:   BTreeMap<u64, ChunkRequest>,
    /// Already-generated positions (no re-request).
    generated: BTreeSet<ChunkPos>,
}

impl ChunkStreamer {
    pub fn new() -> Self {
        Self { pending: BTreeMap::new(), generated: BTreeSet::new() }
    }

    /// Request generation of a single chunk.
    ///
    /// `viewer_pos` is used to compute distance-based priority.
    /// If `pos` has already been generated, the request is ignored.
    pub fn request(&mut self, pos: ChunkPos, biome_name: impl Into<String>, viewer_pos: ChunkPos) {
        if self.generated.contains(&pos) { return; }

        let priority = manhattan_distance(pos, viewer_pos);
        let key      = priority * 1_000_000_000 + pos_hash(pos);

        // Don't overwrite a closer-priority existing request for the same pos
        self.pending.retain(|_, req| req.pos != pos || req.priority <= priority);
        if !self.pending.values().any(|r| r.pos == pos) {
            self.pending.insert(key, ChunkRequest { pos, biome_name: biome_name.into(), priority });
        }
    }

    /// Request a cuboid region of chunks with a biome resolver.
    ///
    /// `biome_fn` maps a `ChunkPos` to the biome name to use for that position.
    pub fn request_region(
        &mut self,
        center:   ChunkPos,
        radius:   i32,
        biome_fn: &dyn Fn(ChunkPos) -> String,
    ) {
        for dy in -radius..=radius {
            for dz in -radius..=radius {
                for dx in -radius..=radius {
                    let p = ChunkPos::new(center.x + dx, center.y + dy, center.z + dz);
                    let biome = biome_fn(p);
                    self.request(p, biome, center);
                }
            }
        }
    }

    /// Pop the highest-priority (closest) pending request.
    pub fn next(&mut self) -> Option<ChunkRequest> {
        let key = *self.pending.keys().next()?;
        let req = self.pending.remove(&key)?;
        Some(req)
    }

    /// Pop up to `max` requests in priority order.
    pub fn drain(&mut self, max: usize) -> Vec<ChunkRequest> {
        let mut out = Vec::with_capacity(max);
        while out.len() < max {
            match self.next() {
                Some(req) => out.push(req),
                None      => break,
            }
        }
        out
    }

    /// Mark a chunk as generated — it will not be requested again.
    pub fn mark_generated(&mut self, pos: ChunkPos) {
        self.generated.insert(pos);
    }

    pub fn pending_count(&self)   -> usize { self.pending.len() }
    pub fn generated_count(&self) -> usize { self.generated.len() }
    pub fn is_empty(&self)        -> bool  { self.pending.is_empty() }
}

impl Default for ChunkStreamer {
    fn default() -> Self { Self::new() }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn manhattan_distance(a: ChunkPos, b: ChunkPos) -> u64 {
    let dx = (a.x as i64 - b.x as i64).unsigned_abs();
    let dy = (a.y as i64 - b.y as i64).unsigned_abs();
    let dz = (a.z as i64 - b.z as i64).unsigned_abs();
    dx + dy + dz
}

/// Cheap deterministic hash of ChunkPos for BTreeMap key tie-breaking.
fn pos_hash(p: ChunkPos) -> u64 {
    let x = p.x as u64;
    let y = p.y as u64;
    let z = p.z as u64;
    x.wrapping_mul(73856093)
        .wrapping_add(y.wrapping_mul(19349663))
        .wrapping_add(z.wrapping_mul(83492791))
        % 1_000_000_000
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn origin() -> ChunkPos { ChunkPos::default() }
    fn at(x: i32, z: i32) -> ChunkPos { ChunkPos::new(x, 0, z) }

    #[test]
    fn request_and_drain() {
        let mut s = ChunkStreamer::new();
        s.request(at(0, 0), "plains", origin());
        s.request(at(1, 0), "forest", origin());
        assert_eq!(s.pending_count(), 2);
        let batch = s.drain(10);
        assert_eq!(batch.len(), 2);
        assert!(s.is_empty());
    }

    #[test]
    fn closest_first_ordering() {
        let viewer = origin();
        let mut s = ChunkStreamer::new();
        s.request(at(5, 5), "plains", viewer); // far
        s.request(at(1, 0), "plains", viewer); // near
        s.request(at(3, 0), "plains", viewer); // medium
        let first = s.next().unwrap();
        // Nearest chunk should come first
        assert_eq!(first.pos, at(1, 0), "expected nearest chunk first");
    }

    #[test]
    fn deduplication() {
        let mut s = ChunkStreamer::new();
        s.request(at(2, 2), "plains", origin());
        s.request(at(2, 2), "forest", origin()); // duplicate pos
        assert_eq!(s.pending_count(), 1, "duplicate should be ignored");
    }

    #[test]
    fn generated_not_re_requested() {
        let mut s = ChunkStreamer::new();
        s.mark_generated(at(3, 3));
        s.request(at(3, 3), "plains", origin());
        assert_eq!(s.pending_count(), 0, "generated chunk should not be re-requested");
    }

    #[test]
    fn request_region() {
        let mut s = ChunkStreamer::new();
        let viewer = ChunkPos::new(5, 0, 5);
        s.request_region(viewer, 2, &|_p| "plains".to_string());
        // 5×5×5 = 125 chunks (radius 2 = -2..=2 in each dim)
        assert_eq!(s.pending_count(), 5*5*5);
    }
}
