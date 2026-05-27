# bKG — Bifrost MMO

> **bKG** · *best known Garbage* · Distributed Reality Compute Fabric

> **same seed = same world** — always, everywhere, forever.

A competitive voxel MMO built on a deterministic distributed simulation engine.
Human factions and a strategic AI civilization compete for territory, resources,
and world dominance across discrete competitive epochs.

---

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────────┐
│                     BIFROST SERVER                      │
│  Rust · deterministic · event-sourced · BLAKE3-hashed   │
├──────────┬──────────┬──────────┬──────────┬─────────────┤
│ Lockstep │ Witness  │   WAC    │   Run    │  Synthesis  │
│   Tick   │ Quorum   │Compiler  │ Director │  AI (Synth) │
├──────────┴──────────┴──────────┴──────────┴─────────────┤
│                    NEXUS VOXEL KERNEL                   │
│   Greedy meshing · NavMesh · Biome generator · WAC RT   │
├─────────────────────────────────────────────────────────┤
│                  BIFROST CLIENT RUNTIME                 │
│   ECS · WebGPU renderer · AnimFSM · Input abstraction   │
├─────────────────────────────────────────────────────────┤
│                    GAME CLIENT                          │
│   app/game.html · isometric 2.5D · Canvas/WebGPU        │
└─────────────────────────────────────────────────────────┘
```

---

## Workspace Crates

### Bifrost Layer — Networking & World Authority

| Crate | Purpose |
|---|---|
| `bifrost-vis` | Voxel Instruction Set — deterministic opcode system (FILL_BOX, SIM_EXPLOSION, …) |
| `bifrost-chunk` | Chunk Authority Epochs — spatial peer authority rotation |
| `bifrost-lockstep` | Lockstep Tick Scheduler — all peers advance in lock-step |
| `bifrost-witness` | Witness Quorum — 1 authority + 2 witnesses, BLAKE3 consensus |
| `bifrost-physics` | Deterministic Physics Kernel — f64, BTreeMap, no SystemTime |
| `bifrost-wac` | World Asset Compiler — LLM/designer intent → validated asset IR |
| `bifrost-aigm` | AI Game Master — NPC AI, quest system, story engine, LLM dialogue |
| `bifrost-run` | World Run System — discrete competitive epochs, win conditions, meta progression |
| `bifrost-synthesis` | Synthesis AI Civilization — strategic AI faction competing against players |
| `bifrost-safe-city` | Safe City + Economy — auction house, zone control, crafting laws |
| `bifrost-server` | HTTP REST server — all bifrost systems exposed over 40+ routes |

### Nexus Layer — Voxel World Generation

| Crate | Purpose |
|---|---|
| `nexus-voxel-kernel` | Greedy meshing, NavMesh (A*), fBm noise, biome generator, WAC runtime adapter |

### Bifrost Client Runtime (nova-* crates)

| Crate | Purpose |
|---|---|
| `nova-core` | ECS World, Transform3D (Vec3/Quat/Mat4), SceneGraph, Timer |
| `nova-render` | WebGPU pipeline — GpuVoxelVertex, Camera3D, WGSL shaders (Phong + AO + fog) |
| `nova-anim` | VoxelSkeleton, AnimClip (slerp), AnimFSM (idle/walk/attack/hurt/die) |
| `nova-input` | KeyCode/MouseButton → ActionId abstraction, InputMap::default_mmo() |

---

## Core Invariant

The server is **only** authoritative over:

| Responsibility | Central |
|---|---|
| Identity / Auth | YES |
| Ledger / Replay | YES |
| Epoch Authority | YES |
| Cheat Detection | YES |
| Global Time | YES |
| Settlement | YES |
| Physics | NO — swarm-computed |
| Rendering | NO — client-computed |
| NPC cognition | NO — distributed agents |

> More players = more compute = more simulation capacity.
> The player population **is** the supercomputer.

---

## Quick Start

```bash
# Build + run (Rust only)
cargo run -p bifrost-server

# Or with Docker
docker build -t nova-mmo . && docker run -p 8080:8080 nova-mmo

# Open game
open http://localhost:8080
```

See [`docs/USAGE.md`](docs/USAGE.md) for the full API reference.

---

## Documentation

| File | Contents |
|---|---|
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | System architecture, authority model, hierarchical simulation |
| [`docs/BIFROST-SPEC.md`](docs/BIFROST-SPEC.md) | Bifrost protocol: VIS opcodes, chunk epochs, lockstep, witness quorum |
| [`docs/WAC.md`](docs/WAC.md) | World Asset Compiler — pipeline, asset types, hard rules |
| [`docs/WORLD.md`](docs/WORLD.md) | World design: run system, factions, economy, safe city, biome evolution |
| [`docs/FACTION.md`](docs/FACTION.md) | Synthesis AI civilization — design, strategy, symmetry guarantee |
| [`docs/NOVA.md`](docs/NOVA.md) | Bifrost client runtime — ECS, WebGPU renderer, animation system |
| [`docs/PLAN.md`](docs/PLAN.md) | Phase roadmap and crate status |
| [`docs/USAGE.md`](docs/USAGE.md) | API reference, curl examples, environment variables |
| [`docs/DOCKER.md`](docs/DOCKER.md) | Docker / BuildKit troubleshooting |
