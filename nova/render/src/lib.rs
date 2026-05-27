//! # nova-render
//!
//! NOVA Engine WebGPU rendering layer.
//!
//! | Module | What it provides |
//! |---|---|
//! | [`camera`] | [`Camera3D`] — perspective/isometric, orbit, view-proj matrix |
//! | [`shaders`] | WGSL source strings: `VOXEL_SHADER`, `SKY_SHADER`, `UI_SHADER` |
//! | [`pipeline`] | [`GpuVoxelVertex`], [`mesh_to_gpu`], [`ChunkMeshRegistry`] |
//!
//! ## Architecture
//!
//! ```text
//! nexus-voxel-kernel                 nova-render
//! ──────────────────                 ────────────────────────────────────
//! VoxelChunk
//!   │  build_mesh()                  WebGPU VoxelPass
//!   ▼                                ├── VertexBuffer  (GpuVoxelVertex ×N)
//! VoxelMesh ──── mesh_to_gpu() ────► ├── IndexBuffer   (u32 ×M)
//!  .positions: Vec<[f32;3]>          ├── voxel.wgsl
//!  .normals:   Vec<[f32;3]>          │   ├── Phong diffuse + ambient
//!  .colors:    Vec<[u8;4]>           │   ├── Fake AO (bottom-face darkening)
//!  .indices:   Vec<u32>              │   └── Distance fog
//!                                    └── ChunkMeshRegistry (uploaded chunks)
//! ```
//!
//! ## Integration with WAC TileMap
//!
//! The `bifrost-wac` crate generates [`TileMapIR`] from natural-language specs.
//! `nova-render` turns that IR into a flat quad mesh for the 2-D tile renderer
//! in `game.html`, keeping Rust and JS tile palettes in sync.
//!
//! ## WebGPU compatibility
//!
//! GPU object creation (device, queue, buffers, pipelines) happens at runtime
//! inside the browser.  This crate only defines the **data layout** and
//! **shader sources** — it compiles on any platform without a GPU.

pub mod camera;
pub mod pipeline;
pub mod shaders;

pub use camera::{Camera3D, CameraMode};
pub use pipeline::{mesh_to_gpu, ChunkMeshRegistry, GpuVoxelVertex};
pub use shaders::{SKY_SHADER, UI_SHADER, VOXEL_SHADER};
