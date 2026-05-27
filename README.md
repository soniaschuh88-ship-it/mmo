# bKG — Bifrost MMO

> **bKG** · *best known Garbage* · Distributed Reality Compute Fabric

> **same seed = same world** — always, everywhere, forever.

A competitive voxel MMO built on a deterministic distributed simulation engine.
Human factions and a strategic AI civilization compete across discrete world epochs.

---

## Stack

```
┌─────────────────────────────────────────────────────────┐
│                  BIFROST SERVER (Rust)                  │
│  lockstep · witness · physics · wac · aigm              │
│  run · synthesis · safe-city · server (40+ HTTP routes) │
├─────────────────────────────────────────────────────────┤
│               NEXUS VOXEL KERNEL (Rust)                 │
│      greedy meshing · NavMesh · biome gen · WAC RT      │
├─────────────────────────────────────────────────────────┤
│          BIFROST CLIENT RUNTIME (nova-* crates)         │
│        ECS · WebGPU · AnimFSM · Camera3D · Input        │
├─────────────────────────────────────────────────────────┤
│                  GAME CLIENT (JS)                       │
│         app/game.html · isometric · Canvas/WebGPU       │
└─────────────────────────────────────────────────────────┘
```

---

## Crates

### Bifrost — Networking & World Authority

| Crate | Purpose |
|---|---|
| `bifrost-vis` | Voxel Instruction Set — deterministic opcodes |
| `bifrost-chunk` | Chunk Authority Epochs |
| `bifrost-lockstep` | Lockstep Tick Scheduler |
| `bifrost-witness` | Witness Quorum (1 authority + 2 witnesses, BLAKE3) |
| `bifrost-physics` | Deterministic Physics (f64, BTreeMap, no SystemTime) |
| `bifrost-wac` | World Asset Compiler + Canonical Biome Registry |
| `bifrost-aigm` | AI Game Master — NPC AI, quests, story, LLM dialogue |
| `bifrost-run` | World Run System — epochs, win conditions, meta progression |
| `bifrost-synthesis` | Synthesis AI Civilization |
| `bifrost-safe-city` | Safe City + Auction House + Zone Control |
| `bifrost-server` | HTTP REST server — 40+ routes |

### Nexus — Voxel World

| Crate | Purpose |
|---|---|
| `nexus-voxel-kernel` | Greedy meshing, NavMesh (A*), fBm noise, biome gen, WAC RT |

### Client Runtime (nova-* crates)

| Crate | Purpose |
|---|---|
| `nova-core` | ECS World, Transform3D (Vec3/Quat/Mat4), SceneGraph, Timer |
| `nova-render` | WebGPU — GpuVoxelVertex, Camera3D, WGSL shaders |
| `nova-anim` | VoxelSkeleton, AnimClip (slerp), AnimFSM |
| `nova-input` | KeyCode/MouseButton → ActionId, InputMap |

---

## Quick Start

```bash
cargo run -p bifrost-server         # server on :8080
open http://localhost:8080          # game client
```

Or with Docker — see [`docs/ops/docker.md`](docs/ops/docker.md).

---

## Docs

```
docs/
├── engine/
│   ├── architecture.md        system design, authority model
│   ├── bifrost-protocol.md    VIS opcodes, epochs, lockstep, witness
│   ├── wac.md                 World Asset Compiler pipeline
│   └── client-runtime.md      ECS, WebGPU, AnimFSM, Input
├── game/
│   ├── world.md               run system, zones, economy, biome evolution
│   ├── factions.md            Synthesis AI civilization design
│   ├── players.md             classes, clone system, meta progression
│   ├── npcs.md                AI Game Master, behavior layers, dialogue
│   ├── monsters.md            types, stats, drops, boss mechanics
│   ├── quests.md              quest chains, objectives, server authority
│   └── skills.md              class skill trees, world manipulation
├── api/
│   └── usage.md               API reference + curl examples (40+ routes)
├── ops/
│   └── docker.md              Docker / BuildKit troubleshooting
└── planning/
    └── roadmap.md             crate status, open PRs, phase roadmap
```
