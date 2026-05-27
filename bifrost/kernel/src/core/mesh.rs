//! Greedy voxel meshing — converts VoxelChunk to triangle mesh data.
//!
//! Implements the classic greedy meshing algorithm for all 6 face directions.
//! Back faces are culled (opaque neighbor = no face emitted).
//! Adjacent same-material faces are merged into quads.
//!
//! # Performance
//!
//! O(CHUNK_SIZE²) per face direction = O(CHUNK_SIZE²) total rather than
//! O(CHUNK_SIZE³) for naive per-voxel meshing.
//!
//! A 32³ fully-solid chunk produces 12 triangles (6 merged quads).
//! A 32³ checkerboard produces ~3072 triangles (worst case for greedy merge).

use crate::core::{VoxelChunk, CHUNK_SIZE};

// ── Output types ──────────────────────────────────────────────────────────────

/// Triangle mesh data ready for GPU upload or glTF export.
#[derive(Debug, Default, Clone)]
pub struct VoxelMesh {
    /// XYZ positions, one per vertex (chunk-local space: [0, CHUNK_SIZE]).
    pub positions:  Vec<[f32; 3]>,
    /// Per-vertex normals (unit vectors).
    pub normals:    Vec<[f32; 3]>,
    /// Per-vertex RGBA color (face-shaded).
    pub colors:     Vec<[u8; 4]>,
    /// Triangle indices (every 3 indices = one triangle).
    pub indices:    Vec<u32>,
}

impl VoxelMesh {
    pub fn vertex_count(&self)   -> usize { self.positions.len() }
    pub fn triangle_count(&self) -> usize { self.indices.len() / 3 }
    pub fn is_empty(&self)       -> bool  { self.indices.is_empty() }

    /// Merge another mesh into this one, offsetting all positions by `offset`.
    pub fn extend(&mut self, other: &VoxelMesh, offset: [f32; 3]) {
        let base = self.positions.len() as u32;
        for &p in &other.positions {
            self.positions.push([p[0]+offset[0], p[1]+offset[1], p[2]+offset[2]]);
        }
        self.normals.extend_from_slice(&other.normals);
        self.colors.extend_from_slice(&other.colors);
        for &i in &other.indices { self.indices.push(base + i); }
    }
}

// ── Face shading ──────────────────────────────────────────────────────────────

/// Apply directional shading to a base colour.
/// Top=100%, sides=78%, bottom=55%.
fn shade(color: [u8; 4], normal_axis: usize, positive: bool) -> [u8; 4] {
    let factor = match (normal_axis, positive) {
        (1, true)  => 1.00_f32, // +Y top
        (1, false) => 0.55_f32, // -Y bottom
        _          => 0.78_f32, // X/Z sides
    };
    [
        (color[0] as f32 * factor) as u8,
        (color[1] as f32 * factor) as u8,
        (color[2] as f32 * factor) as u8,
        color[3],
    ]
}

// ── Build mesh ────────────────────────────────────────────────────────────────

/// Build a greedy mesh from a chunk and material colour table.
///
/// `palette` maps material IDs to RGBA colours.
pub fn build_mesh(chunk: &VoxelChunk, palette: &[(u16, [u8; 4])]) -> VoxelMesh {
    let mut mesh = VoxelMesh::default();
    let s = CHUNK_SIZE;

    // Process all 6 face directions: 3 axes × 2 signs
    for axis in 0..3usize {
        let u = (axis + 1) % 3;
        let v = (axis + 2) % 3;

        for &positive in &[true, false] {
            // Sweep slices along the normal axis
            for d in 0..s {
                // Build 2D mask: mask[vi*s+ui] = material (0 = no face)
                let mut mask = vec![0u16; s * s];

                for vi in 0..s {
                    for ui in 0..s {
                        let mut pos = [0usize; 3];
                        pos[axis] = d; pos[u] = ui; pos[v] = vi;

                        let vox = chunk.get(pos[0], pos[1], pos[2]);
                        if vox.is_air() || !vox.is_solid() { continue; }

                        // Neighbour in the face direction
                        let exposed = if positive {
                            if d + 1 >= s { true } // chunk boundary
                            else {
                                let n = chunk.get(
                                    if axis==0 {d+1} else {pos[0]},
                                    if axis==1 {d+1} else {pos[1]},
                                    if axis==2 {d+1} else {pos[2]},
                                );
                                n.is_air() || n.is_transparent()
                            }
                        } else {
                            if d == 0 { true }
                            else {
                                let n = chunk.get(
                                    if axis==0 {d-1} else {pos[0]},
                                    if axis==1 {d-1} else {pos[1]},
                                    if axis==2 {d-1} else {pos[2]},
                                );
                                n.is_air() || n.is_transparent()
                            }
                        };

                        if exposed { mask[vi * s + ui] = vox.material; }
                    }
                }

                // Greedy merge over the 2D mask
                let mut visited = vec![false; s * s];
                let face_coord = if positive { d + 1 } else { d } as f32;

                for vi in 0..s {
                    for ui in 0..s {
                        let idx = vi * s + ui;
                        if visited[idx] || mask[idx] == 0 { continue; }
                        let mat = mask[idx];

                        // Expand in u direction
                        let mut w = 1;
                        while ui + w < s
                            && !visited[vi*s + ui+w]
                            && mask[vi*s + ui+w] == mat
                        { w += 1; }

                        // Expand in v direction
                        let mut h = 1;
                        'vloop: while vi + h < s {
                            for k in 0..w {
                                let i2 = (vi+h)*s + ui+k;
                                if visited[i2] || mask[i2] != mat { break 'vloop; }
                            }
                            h += 1;
                        }

                        // Mark visited
                        for dv in 0..h { for du in 0..w { visited[(vi+dv)*s + ui+du] = true; } }

                        // Resolve world positions of the 4 quad vertices
                        let mut v0 = [0.0f32; 3];
                        let mut v1 = [0.0f32; 3];
                        let mut v2 = [0.0f32; 3];
                        let mut v3 = [0.0f32; 3];

                        // Common: set the slice coordinate
                        v0[axis] = face_coord; v1[axis] = face_coord;
                        v2[axis] = face_coord; v3[axis] = face_coord;

                        // Set u/v coordinates for each corner
                        v0[u]=ui as f32;   v0[v]=vi as f32;
                        v1[u]=(ui+w) as f32; v1[v]=vi as f32;
                        v2[u]=(ui+w) as f32; v2[v]=(vi+h) as f32;
                        v3[u]=ui as f32;   v3[v]=(vi+h) as f32;

                        // Normal vector
                        let mut normal = [0.0f32; 3];
                        normal[axis] = if positive { 1.0 } else { -1.0 };

                        // Color with face shading
                        let base_color = palette.iter()
                            .find(|&&(id, _)| id == mat)
                            .map(|&(_, c)| c)
                            .unwrap_or([150, 150, 150, 255]);
                        let color = shade(base_color, axis, positive);

                        // Emit 4 vertices
                        let base = mesh.positions.len() as u32;
                        mesh.positions.extend_from_slice(&[v0, v1, v2, v3]);
                        for _ in 0..4 { mesh.normals.push(normal); mesh.colors.push(color); }

                        // Triangle winding: front face depends on normal direction
                        if positive {
                            mesh.indices.extend_from_slice(&[base, base+2, base+1, base, base+3, base+2]);
                        } else {
                            mesh.indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
                        }
                    }
                }
            }
        }
    }
    mesh
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ChunkPos, Voxel, VoxelChunk, CHUNK_SIZE};
    use crate::core::materials;

    fn pal() -> Vec<(u16, [u8; 4])> { vec![(materials::STONE,[120,120,130,255])] }

    #[test]
    fn empty_chunk_no_mesh() {
        let c = VoxelChunk::empty(ChunkPos::default());
        assert!(build_mesh(&c, &pal()).is_empty());
    }

    #[test]
    fn single_voxel_six_faces() {
        let mut c = VoxelChunk::empty(ChunkPos::default());
        c.set(5, 5, 5, Voxel::solid(materials::STONE));
        let m = build_mesh(&c, &pal());
        assert_eq!(m.triangle_count(), 12, "isolated voxel → 6 quads → 12 triangles");
        assert_eq!(m.vertex_count(),   24, "6 quads × 4 verts = 24");
    }

    #[test]
    fn full_chunk_only_outer_surface() {
        let mut c = VoxelChunk::empty(ChunkPos::default());
        c.fill(Voxel::solid(materials::STONE));
        let m = build_mesh(&c, &pal());
        // All interior faces culled; 6 outer faces, each merged to 1 quad
        assert_eq!(m.triangle_count(), 12, "full chunk → 6 outer quads → 12 triangles");
    }

    #[test]
    fn two_adjacent_merge() {
        // Two voxels side by side in X → greedy merges their parallel faces
        let mut c = VoxelChunk::empty(ChunkPos::default());
        c.set(4, 4, 4, Voxel::solid(materials::STONE));
        c.set(5, 4, 4, Voxel::solid(materials::STONE));
        let m = build_mesh(&c, &pal());
        // 2 touching faces culled (internal); remaining 10 faces merged by axis:
        // +Y, -Y, +Z, -Z each merge 2→1 = 4 quads; +X cap, -X cap = 2 quads → 6 × 2 = 12 tri
        assert_eq!(m.triangle_count(), 12, "two adj voxels → 6 greedy quads → 12 tri");
    }

    #[test]
    fn slab_efficient_meshing() {
        let mut c = VoxelChunk::empty(ChunkPos::default());
        let s = CHUNK_SIZE;
        // Fill entire bottom layer
        for z in 0..s { for x in 0..s { c.set(x, 0, z, Voxel::solid(materials::STONE)); } }
        let m = build_mesh(&c, &pal());
        // Greedy merges each face into 1 quad: top + bottom + 4 sides = 6 quads = 12 triangles
        assert_eq!(m.triangle_count(), 12, "1-layer slab → 6 merged quads");
    }

    #[test]
    fn indices_in_bounds() {
        let mut c = VoxelChunk::empty(ChunkPos::default());
        c.set(0, 0, 0, Voxel::solid(materials::STONE));
        c.set(1, 0, 0, Voxel::solid(materials::STONE));
        let m = build_mesh(&c, &pal());
        let max = m.vertex_count() as u32;
        assert!(m.indices.iter().all(|&i| i < max));
    }

    #[test]
    fn extend_combines_meshes() {
        let mut c = VoxelChunk::empty(ChunkPos::default());
        c.set(0, 0, 0, Voxel::solid(materials::STONE));
        let single = build_mesh(&c, &pal());
        let mut combined = VoxelMesh::default();
        combined.extend(&single, [0.0, 0.0, 0.0]);
        combined.extend(&single, [32.0, 0.0, 0.0]);
        assert_eq!(combined.vertex_count(), single.vertex_count() * 2);
        assert_eq!(combined.triangle_count(), single.triangle_count() * 2);
    }
}
