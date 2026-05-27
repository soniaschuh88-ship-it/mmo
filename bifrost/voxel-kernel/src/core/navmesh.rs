//! Navigation mesh generator — converts a VoxelChunk into a pathfinding graph.
//!
//! A position (x, y, z) is **navigable** when:
//! 1. The voxel at (x, y, z) is solid (the floor).
//! 2. The voxel at (x, y+1, z) is air or transparent (standing room).
//! 3. The voxel at (x, y+2, z) is also clear (full-height clearance).
//!
//! Adjacency connects navigable positions that are horizontally adjacent
//! (4-connected) or one step up/down (slope ≤ 1 voxel).
//!
//! # Usage
//!
//! ```rust,ignore
//! let chunk = generate_chunk(pos, &biome);
//! let origin = pos.world_origin();
//! let nav = build_navmesh(&chunk, origin);
//!
//! // Find a path from A to B
//! let path = nav.find_path(start, goal);
//! ```

use std::collections::{BTreeMap, BinaryHeap};
use std::cmp::Reverse;

use serde::{Deserialize, Serialize};

use crate::core::{VoxelChunk, CHUNK_SIZE};

// ── NavFlags ──────────────────────────────────────────────────────────────────

pub mod nav_flags {
    /// Tile is passable under normal conditions.
    pub const PASSABLE:  u8 = 0b0000_0001;
    /// Floor is a liquid (water, lava) — passable but costly.
    pub const WET:       u8 = 0b0000_0010;
    /// Steep incline (height diff = 1) — costs more to traverse.
    pub const SLOPE:     u8 = 0b0000_0100;
    /// Emissive floor (crystal, glowstone) — affects lighting but not movement.
    pub const LIT:       u8 = 0b0000_1000;
    /// Hazardous material (lava) — high movement cost.
    pub const HAZARD:    u8 = 0b0001_0000;
}

// ── NavNode ───────────────────────────────────────────────────────────────────

/// A single node in the navigation graph — one navigable floor tile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavNode {
    /// World-space position of the floor voxel (y = floor level).
    pub world_pos: (i64, i64, i64),
    /// Floor material ID (affects movement cost).
    pub material:  u16,
    /// Navigation flags.
    pub flags:     u8,
}

impl NavNode {
    pub fn is_passable(&self) -> bool { self.flags & nav_flags::PASSABLE != 0 }
    pub fn is_wet(&self)      -> bool { self.flags & nav_flags::WET     != 0 }
    pub fn is_hazard(&self)   -> bool { self.flags & nav_flags::HAZARD  != 0 }
}

// ── NavEdge ───────────────────────────────────────────────────────────────────

/// A directed edge in the navigation graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavEdge {
    /// Target node index in `NavMesh::nodes`.
    pub to:   usize,
    /// Movement cost (1.0 = normal, higher = slower, 0.0 = free).
    pub cost: f32,
}

// ── NavMesh ───────────────────────────────────────────────────────────────────

/// Navigation graph for a single chunk.
///
/// Node indices are stable — once built, do not add/remove nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavMesh {
    pub nodes:   Vec<NavNode>,
    /// Adjacency list: edges[i] = outgoing edges from node i.
    pub edges:   Vec<Vec<NavEdge>>,
    /// Fast lookup: world floor position → node index.
    pos_map:     BTreeMap<(i64, i64, i64), usize>,
}

impl NavMesh {
    /// Number of navigable nodes.
    pub fn node_count(&self) -> usize { self.nodes.len() }

    /// True if the graph has no navigable tiles.
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }

    /// Outgoing edges from node `i`.
    pub fn neighbors(&self, i: usize) -> &[NavEdge] {
        self.edges.get(i).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Look up the node at a world floor position.
    pub fn node_at(&self, pos: (i64, i64, i64)) -> Option<usize> {
        self.pos_map.get(&pos).copied()
    }

    // ── A* pathfinding ────────────────────────────────────────────────────────

    /// Find the lowest-cost path between two world positions using A*.
    ///
    /// Returns `None` if no path exists or either position is not navigable.
    /// The returned `Vec` includes both start and goal positions.
    pub fn find_path(
        &self,
        start: (i64, i64, i64),
        goal:  (i64, i64, i64),
    ) -> Option<Vec<(i64, i64, i64)>> {
        let start_idx = self.node_at(start)?;
        let goal_idx  = self.node_at(goal)?;

        if start_idx == goal_idx {
            return Some(vec![start]);
        }

        // Priority queue: (Reverse(f_cost), node_idx)
        let mut open: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();
        let mut g_cost = vec![f32::MAX; self.nodes.len()];
        let mut came_from = vec![usize::MAX; self.nodes.len()];

        g_cost[start_idx] = 0.0;
        open.push(Reverse((0, start_idx)));

        let heuristic = |idx: usize| -> f32 {
            let p = self.nodes[idx].world_pos;
            let gp = self.nodes[goal_idx].world_pos;
            let dx = (p.0 - gp.0).abs() as f32;
            let dy = (p.1 - gp.1).abs() as f32;
            let dz = (p.2 - gp.2).abs() as f32;
            dx + dy + dz // Manhattan heuristic
        };

        while let Some(Reverse((_, current))) = open.pop() {
            if current == goal_idx {
                // Reconstruct path
                let mut path = Vec::new();
                let mut cur = goal_idx;
                while cur != usize::MAX {
                    path.push(self.nodes[cur].world_pos);
                    cur = came_from[cur];
                }
                path.reverse();
                return Some(path);
            }

            for edge in self.neighbors(current) {
                let tentative = g_cost[current] + edge.cost;
                if tentative < g_cost[edge.to] {
                    g_cost[edge.to] = tentative;
                    came_from[edge.to] = current;
                    let f = tentative + heuristic(edge.to);
                    open.push(Reverse(((f * 1000.0) as u32, edge.to)));
                }
            }
        }
        None // no path found
    }

    /// Straight-line distance between two world positions.
    pub fn path_cost(path: &[(i64, i64, i64)]) -> f32 {
        if path.len() < 2 { return 0.0; }
        path.windows(2).map(|w| {
            let dx = (w[1].0 - w[0].0).abs() as f32;
            let dy = (w[1].1 - w[0].1).abs() as f32;
            let dz = (w[1].2 - w[0].2).abs() as f32;
            (dx*dx + dy*dy + dz*dz).sqrt()
        }).sum()
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a navigation mesh from a chunk.
///
/// `world_origin` is the world-space coordinate of the chunk's (0,0,0) corner.
pub fn build_navmesh(chunk: &VoxelChunk, world_origin: (i64, i64, i64)) -> NavMesh {
    let s  = CHUNK_SIZE;
    let wo = world_origin;
    let mut nodes:   Vec<NavNode>          = Vec::new();
    let mut edges:   Vec<Vec<NavEdge>>     = Vec::new();
    let mut pos_map: BTreeMap<(i64,i64,i64), usize> = BTreeMap::new();

    // Pass 1: identify all navigable floor positions
    for z in 0..s {
        for y in 0..s {
            for x in 0..s {
                let floor = chunk.get(x, y, z);
                if floor.is_air() || !floor.is_solid() { continue; }

                // Check standing room (y+1 and y+2)
                let clear1 = y + 1 < s && !chunk.get(x, y+1, z).is_solid();
                let clear2 = if y + 2 < s { !chunk.get(x, y+2, z).is_solid() } else { true };
                if !(clear1 && clear2) { continue; }

                let world_pos = (wo.0 + x as i64, wo.1 + y as i64, wo.2 + z as i64);

                // Compute flags
                let mut flags = nav_flags::PASSABLE;
                if floor.is_emissive() { flags |= nav_flags::LIT; }

                // Material-based flags
                use crate::core::materials;
                if floor.material == materials::WATER || floor.material == materials::LAVA {
                    flags |= nav_flags::WET;
                    if floor.material == materials::LAVA { flags |= nav_flags::HAZARD; }
                }

                let idx = nodes.len();
                nodes.push(NavNode { world_pos, material: floor.material, flags });
                edges.push(Vec::new());
                pos_map.insert(world_pos, idx);
            }
        }
    }

    // Pass 2: connect adjacent navigable nodes
    let n = nodes.len();
    // Collect positions to avoid borrow issues
    let positions: Vec<(i64,i64,i64)> = nodes.iter().map(|n| n.world_pos).collect();

    for i in 0..n {
        let (px, py, pz) = positions[i];
        let base_cost = material_cost(nodes[i].material);

        // 4-connected horizontal neighbours (±X, ±Z) + allow ±1 Y step
        for (dx, dz) in [(-1,0),(1,0),(0,-1),(0,1)] {
            let nx = px + dx;
            let nz = pz + dz;
            // Try same level, step up, step down
            for dy in [-1i64, 0, 1] {
                let ny = py + dy;
                if let Some(&j) = pos_map.get(&(nx, ny, nz)) {
                    let target_cost = material_cost(nodes[j].material);
                    let mut cost = (base_cost + target_cost) * 0.5;
                    if dy != 0 { cost *= 1.4; } // slope penalty
                    if nodes[j].is_hazard() { cost *= 5.0; }
                    edges[i].push(NavEdge { to: j, cost });
                }
            }
        }
    }

    NavMesh { nodes, edges, pos_map }
}

/// Movement cost for a floor material (1.0 = normal, >1 = slower).
fn material_cost(material: u16) -> f32 {
    use crate::core::materials;
    match material {
        materials::WATER  => 3.0,
        materials::SAND   => 1.4,
        materials::GRAVEL => 1.2,
        materials::ICE    => 0.7,  // faster (slippery)
        materials::LAVA   => 10.0,
        materials::MOSS   => 1.1,
        _                 => 1.0,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ChunkPos, Voxel, VoxelChunk, CHUNK_SIZE};
    use crate::core::materials;

    fn origin() -> (i64,i64,i64) { (0,0,0) }

    fn chunk_with_floor(y: usize) -> VoxelChunk {
        let mut c = VoxelChunk::empty(ChunkPos::default());
        let s = CHUNK_SIZE;
        for z in 0..s { for x in 0..s { c.set(x, y, z, Voxel::solid(materials::STONE)); } }
        c
    }

    #[test]
    fn empty_chunk_no_nodes() {
        let c = VoxelChunk::empty(ChunkPos::default());
        let nav = build_navmesh(&c, origin());
        assert!(nav.is_empty());
    }

    #[test]
    fn flat_floor_nodes() {
        let c = chunk_with_floor(0);
        let nav = build_navmesh(&c, origin());
        // 32×32 floor at y=0 (needs y+1 and y+2 clear — they are air)
        assert_eq!(nav.node_count(), CHUNK_SIZE * CHUNK_SIZE,
            "flat floor should produce {}² nodes", CHUNK_SIZE);
    }

    #[test]
    fn blocked_floor_no_nodes() {
        // Fill all voxels solid → nowhere to stand → zero nav nodes
        let mut c = VoxelChunk::empty(ChunkPos::default());
        c.fill(Voxel::solid(materials::STONE));
        let nav = build_navmesh(&c, origin());
        assert_eq!(nav.node_count(), 0,
            "completely filled chunk has no standing room → no nav nodes");
    }

    #[test]
    fn adjacency_connected() {
        let c = chunk_with_floor(0);
        let nav = build_navmesh(&c, origin());
        // Each interior node should have 4 neighbours
        // Node at (1,0,1) → check its edge count
        let p = (1i64, 0i64, 1i64);
        let idx = nav.node_at(p).expect("(1,0,1) should be navigable");
        assert_eq!(nav.neighbors(idx).len(), 4,
            "interior tile should have 4 horizontal edges");
    }

    #[test]
    fn corner_node_has_two_edges() {
        let c = chunk_with_floor(0);
        let nav = build_navmesh(&c, origin());
        // Corner node (0,0,0) has only 2 horizontal neighbours
        let idx = nav.node_at((0, 0, 0)).unwrap();
        assert_eq!(nav.neighbors(idx).len(), 2,
            "corner tile should have 2 edges");
    }

    #[test]
    fn pathfinding_straight_line() {
        let c = chunk_with_floor(0);
        let nav = build_navmesh(&c, origin());
        let start = (0i64, 0, 0);
        let goal  = (5i64, 0, 0);
        let path = nav.find_path(start, goal).expect("path should exist");
        assert_eq!(*path.first().unwrap(), start);
        assert_eq!(*path.last().unwrap(), goal);
        assert_eq!(path.len(), 6, "straight line: 6 steps");
    }

    #[test]
    fn pathfinding_unreachable() {
        // Two isolated floor tiles
        let mut c = VoxelChunk::empty(ChunkPos::default());
        c.set(0, 0, 0, Voxel::solid(materials::STONE));
        c.set(5, 0, 5, Voxel::solid(materials::STONE));
        let nav = build_navmesh(&c, origin());
        assert!(nav.find_path((0,0,0), (5,0,5)).is_none(),
            "disconnected tiles have no path");
    }

    #[test]
    fn world_origin_offset() {
        let c = chunk_with_floor(2);
        let wo = (64i64, 0, 128i64);
        let nav = build_navmesh(&c, wo);
        // First node should be at world (64, 2, 128)
        assert!(nav.node_at((64, 2, 128)).is_some(),
            "node positions must include world origin offset");
    }

    #[test]
    fn slope_connectivity() {
        // Two platforms at different heights
        let mut c = VoxelChunk::empty(ChunkPos::default());
        for x in 0..5 { c.set(x, 0, 0, Voxel::solid(materials::STONE)); }
        for x in 5..10 { c.set(x, 1, 0, Voxel::solid(materials::STONE)); }
        let nav = build_navmesh(&c, origin());
        // Should be able to walk up the 1-block step
        let path = nav.find_path((0,0,0), (9,1,0));
        assert!(path.is_some(), "1-block step should be navigable");
    }
}


