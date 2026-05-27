# bKG — Bifrost MMO · Phase Roadmap

> *bKG · best known Garbage · DRCF — Distributed Reality Compute Fabric*

---

## Current State (after PR #3)

| Crate | Status | Description |
|---|---|---|
| `bifrost-vis` | ✅ | Voxel Instruction Set opcodes |
| `bifrost-chunk` | ✅ | Chunk authority epoch system |
| `bifrost-lockstep` | ✅ | Lockstep tick scheduler |
| `bifrost-witness` | ✅ | Witness quorum execution |
| `bifrost-physics` | ✅ | Deterministic f64 physics kernel |
| `bifrost-wac` | ✅ | World Asset Compiler + TileMap + NVIDIA NIM |
| `bifrost-aigm` | ✅ | NPC AI, quest system, story engine |
| `bifrost-run` | ✅ | World Run system, win conditions, meta progression |
| `bifrost-synthesis` | ✅ | Synthesis AI civilization, strategy engine |
| `bifrost-safe-city` | ✅ | Safe City, Auction House, zone control |
| `bifrost-server` | ✅ | HTTP REST API — 40+ routes |
| `nexus-voxel-kernel` | ✅ | Greedy meshing, NavMesh, biome generator, WAC RT |
| `nova-core` | ✅ | ECS, Transform3D, SceneGraph, Timer |
| `nova-render` | ✅ | WebGPU pipeline, Camera3D, WGSL shaders |
| `nova-anim` | ✅ | VoxelSkeleton, AnimClip, AnimFSM |
| `nova-input` | ✅ | KeyCode → ActionId, InputMap::default_mmo() |

---

## Active PRs

| PR | Title | Status |
|---|---|---|
| [#4](https://github.com/soniaschuh88-ship-it/mmo/pull/4) | fix(determinism): noise smoothstep + canonical biome registry | Open |

---

## Drift Fix PRs (in order)

These fix the three-way drift discovered in the codebase audit.

| PR | Steps | Description |
|---|---|---|
| PR 1 (open) | 1+2 | Noise smoothstep JS=Rust + canonical biome registry |
| PR 2 | 3+8 | Unify `BiomeIR` duplicate + `Vec3Payload → nova_core::Vec3` |
| PR 3 | 5+6 | Quest HTTP routes + game.html API fetch (server-authoritative world data) |
| PR 4 | 7 | Wire run-end → WorldDirector (self-evolving world loop) |
| PR 5 | 4 | AnimationGraphIR → nova-anim FSM bridge (WAC as behavior compiler) |

---

## Phase 2 — MMO Fabric Integration

- [ ] Browser WASM compilation of nova-core + nova-render
- [ ] WebGPU world rendering replaces Canvas 2D
- [ ] WebRTC peer mesh bridge (Node.js)
- [ ] GPU P2P distribution network

## Phase 3 — DELPHOS ↔ Bifrost Protocol

- [ ] Formal epoch handshake protocol
- [ ] Ledger-backed chunk replay
- [ ] Settlement finalization bridge

## Phase 4 — Advanced Bifrost

- [ ] `bifrost-lod` — Level-of-detail simulation abstraction
- [ ] `bifrost-combat` — Probabilistic distant combat
- [ ] `bifrost-reputation` — Proof-of-Render trust score engine

---

## Core Invariants (Never Violate)

1. Physics is never centralized
2. Rendering is never centralized
3. NPC cognition is never centralized
4. Combat resolution is never centralized
5. DELPHOS decides truth; players compute reality
6. **Same seed = same world** — always, everywhere
7. Every world concept exists **once** (`bifrost-wac` is the World Type Authority)
