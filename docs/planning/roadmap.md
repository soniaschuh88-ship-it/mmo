# bKG — Bifrost MMO · Roadmap

> *bKG · best known Garbage · DRCF — Distributed Reality Compute Fabric*

---

## Current State

| Crate | Status | Description |
|---|---|---|
| `bifrost-vis` | ✅ | Voxel Instruction Set opcodes |
| `bifrost-chunk` | ✅ | Chunk authority epoch system |
| `bifrost-lockstep` | ✅ | Lockstep tick scheduler |
| `bifrost-witness` | ✅ | Witness quorum execution |
| `bifrost-physics` | ✅ | Deterministic f64 physics kernel |
| `bifrost-wac` | ✅ | World Asset Compiler + canonical biome registry |
| `bifrost-aigm` | ✅ | NPC AI, quest system, story engine |
| `bifrost-run` | ✅ | World Run system, win conditions, meta progression |
| `bifrost-synthesis` | ✅ | Synthesis AI civilization, strategy engine |
| `bifrost-safe-city` | ✅ | Safe City, Auction House, zone control |
| `bifrost-server` | ✅ | HTTP REST API — 40+ routes |
| `nexus-voxel-kernel` | ✅ | Greedy meshing, NavMesh, biome gen, WAC runtime |
| `nova-core` | ✅ | ECS, Transform3D, SceneGraph, Timer |
| `nova-render` | ✅ | WebGPU pipeline, Camera3D, WGSL shaders |
| `nova-anim` | ✅ | VoxelSkeleton, AnimClip, AnimFSM |
| `nova-input` | ✅ | KeyCode → ActionId, InputMap |

---

## Open PRs

| PR | Description | Status |
|---|---|---|
| [#4](https://github.com/soniaschuh88-ship-it/mmo/pull/4) | Noise smoothstep JS=Rust + canonical biome registry | Open |

---

## Drift Fix Sequence

| PR | Steps | Fixes |
|---|---|---|
| PR 1 (open #4) | 1+2 | Noise smoothstep + canonical biome registry |
| PR 2 | 3+8 | Unify `BiomeIR` duplicate + `Vec3Payload → nova_core::Vec3` |
| PR 3 | 5+6 | Quest/NPC HTTP routes + game.html API fetch |
| PR 4 | 7 | Wire run-end → WorldDirector (self-evolving world loop) |
| PR 5 | 4 | `AnimationGraphIR::to_nova_fsm()` bridge |

---

## Phase 2 — MMO Fabric Integration

- [ ] WASM compilation for nova-core + nova-render
- [ ] WebGPU world rendering replaces Canvas 2D
- [ ] WebRTC peer mesh bridge
- [ ] GPU P2P distribution network

## Phase 3 — DELPHOS ↔ Bifrost Protocol

- [ ] Formal epoch handshake protocol
- [ ] Ledger-backed chunk replay
- [ ] Settlement finalization bridge

## Phase 4 — Advanced Bifrost

- [ ] `bifrost-lod` — Level-of-detail simulation abstraction
- [ ] `bifrost-combat` — Probabilistic distant combat
- [ ] `bifrost-reputation` — Proof-of-Render trust score engine
- [ ] `bifrost-economy` — Emergent player-driven economy

---

## Core Invariants

1. Physics is never centralized
2. Rendering is never centralized
3. NPC cognition is never centralized
4. Combat resolution is never centralized
5. DELPHOS decides truth; players compute reality
6. **Same seed = same world** — always, everywhere
7. Every world concept exists **once** (`bifrost-wac` is the World Type Authority)
