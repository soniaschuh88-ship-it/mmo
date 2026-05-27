//! GPU vertex layout, mesh conversion, and chunk registry.
//!
//! This module bridges [`nexus_voxel_kernel::core::VoxelMesh`] (CPU-side
//! greedy mesh) to the WebGPU vertex buffer format used by `VOXEL_SHADER`.

use std::collections::BTreeMap;

use nexus_voxel_kernel::core::{ChunkPos, VoxelMesh};

// ─── GpuVoxelVertex ───────────────────────────────────────────────────────────

/// One vertex in the voxel render pipeline.
///
/// Memory layout (40 bytes, `repr(C)`):
///
/// | Offset | Size | Field    | WGSL         |
/// |--------|------|----------|--------------|
/// |  0     | 12   | position | `vec3<f32>`  |
/// | 12     | 12   | normal   | `vec3<f32>`  |
/// | 24     | 16   | color    | `vec4<f32>`  |
///
/// Derived `Pod + Zeroable` via `bytemuck` — safe to cast to `&[u8]` for upload.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVoxelVertex {
    /// World-space position.
    pub position: [f32; 3],
    /// Surface normal (unit length expected by shader).
    pub normal:   [f32; 3],
    /// RGBA color in `[0, 1]` range (converted from `u8` palette).
    pub color:    [f32; 4],
}

// ─── mesh_to_gpu ──────────────────────────────────────────────────────────────

/// Convert a greedy-meshed [`VoxelMesh`] to flat vertex + index buffers
/// ready for a `wgpu::Buffer`.
///
/// # Arguments
/// * `mesh` — output of [`nexus_voxel_kernel::core::build_mesh`]
///
/// # Returns
/// `(vertices, indices)` where `bytemuck::cast_slice(&vertices)` gives the
/// raw bytes for the vertex buffer.
pub fn mesh_to_gpu(mesh: &VoxelMesh) -> (Vec<GpuVoxelVertex>, Vec<u32>) {
    let verts: Vec<GpuVoxelVertex> = mesh.positions.iter()
        .zip(mesh.normals.iter())
        .zip(mesh.colors.iter())
        .map(|((pos, nrm), col)| GpuVoxelVertex {
            position: *pos,
            normal:   *nrm,
            color: [
                col[0] as f32 / 255.0,
                col[1] as f32 / 255.0,
                col[2] as f32 / 255.0,
                col[3] as f32 / 255.0,
            ],
        })
        .collect();

    (verts, mesh.indices.clone())
}

// ─── Vertex buffer layout descriptor ─────────────────────────────────────────

/// Describes how [`GpuVoxelVertex`] maps to `@location` attributes in WGSL.
///
/// Pass to `wgpu::RenderPipelineDescriptor::vertex.buffers`.
pub struct VtxLayout {
    /// Bytes per vertex.
    pub array_stride: u64,
    pub attributes:   Vec<VtxAttr>,
}

pub struct VtxAttr {
    pub shader_location: u32,
    pub offset:          u64,
    pub format:          VtxFmt,
}

pub enum VtxFmt {
    Float32x3,
    Float32x4,
}

/// Build the vertex buffer layout descriptor for `GpuVoxelVertex`.
pub fn voxel_vertex_layout() -> VtxLayout {
    VtxLayout {
        array_stride: std::mem::size_of::<GpuVoxelVertex>() as u64,
        attributes: vec![
            VtxAttr { shader_location: 0, offset:  0, format: VtxFmt::Float32x3 },
            VtxAttr { shader_location: 1, offset: 12, format: VtxFmt::Float32x3 },
            VtxAttr { shader_location: 2, offset: 24, format: VtxFmt::Float32x4 },
        ],
    }
}

// ─── ChunkMeshRegistry ────────────────────────────────────────────────────────

/// Tracks which chunks have GPU buffers uploaded and which need a re-upload.
///
/// Used by the renderer to avoid redundant buffer uploads when the world
/// has not changed.
#[derive(Default)]
pub struct ChunkMeshRegistry {
    /// `(chunk_pos, lod)` → `(vertex_count, index_count)`
    pub uploaded: BTreeMap<(ChunkPos, u8), (u32, u32)>,
    /// Chunks that have been modified and need re-upload.
    pub dirty:    Vec<ChunkPos>,
}

impl ChunkMeshRegistry {
    /// Mark a chunk as needing a fresh GPU upload.
    pub fn mark_dirty(&mut self, pos: ChunkPos) {
        if !self.dirty.contains(&pos) { self.dirty.push(pos); }
    }

    /// Record that a chunk has been uploaded at `lod` level.
    pub fn register(&mut self, pos: ChunkPos, lod: u8, vertex_count: u32, index_count: u32) {
        self.uploaded.insert((pos, lod), (vertex_count, index_count));
        self.dirty.retain(|p| p != &pos);
    }

    pub fn is_uploaded(&self, pos: ChunkPos, lod: u8) -> bool {
        self.uploaded.contains_key(&(pos, lod))
    }

    /// Total vertex count across all uploaded chunks (useful for GPU memory estimates).
    pub fn total_vertices(&self) -> u64 {
        self.uploaded.values().map(|(v, _)| *v as u64).sum()
    }

    pub fn dirty_count(&self) -> usize { self.dirty.len() }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_voxel_kernel::core::{build_mesh, ChunkPos, Voxel, VoxelChunk, materials};

    fn one_voxel_mesh() -> VoxelMesh {
        let mut chunk = VoxelChunk::empty(ChunkPos::default());
        chunk.set(0, 0, 0, Voxel::solid(materials::STONE));
        let palette = vec![(materials::STONE, [120u8, 120, 130, 255])];
        build_mesh(&chunk, &palette)
    }

    #[test]
    fn mesh_to_gpu_preserves_counts() {
        let mesh = one_voxel_mesh();
        let (verts, indices) = mesh_to_gpu(&mesh);
        assert_eq!(verts.len(), mesh.positions.len());
        assert_eq!(indices.len(), mesh.indices.len());
    }

    #[test]
    fn colors_are_in_unit_range() {
        let mesh = one_voxel_mesh();
        let (verts, _) = mesh_to_gpu(&mesh);
        for v in &verts {
            for &c in &v.color {
                assert!(c >= 0.0 && c <= 1.0, "color component out of range: {c}");
            }
        }
    }

    #[test]
    fn vertex_stride_is_40_bytes() {
        assert_eq!(std::mem::size_of::<GpuVoxelVertex>(), 40);
    }

    #[test]
    fn vertex_layout_has_three_attributes() {
        let layout = voxel_vertex_layout();
        assert_eq!(layout.array_stride, 40);
        assert_eq!(layout.attributes.len(), 3);
    }

    #[test]
    fn chunk_registry_lifecycle() {
        let mut reg = ChunkMeshRegistry::default();
        let pos = ChunkPos::default();

        assert!(!reg.is_uploaded(pos, 0));
        reg.mark_dirty(pos);
        assert_eq!(reg.dirty_count(), 1);

        reg.register(pos, 0, 24, 36);
        assert!(reg.is_uploaded(pos, 0));
        assert_eq!(reg.dirty_count(), 0);      // mark_dirty clears on register
        assert_eq!(reg.total_vertices(), 24);
    }
}
