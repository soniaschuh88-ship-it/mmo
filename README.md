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
│         BIFROST CLIENT RUNTIME (nova-* crates)          │
│       ECS · WebGPU · AnimFSM · Camera3D · Input         │
├─────────────────────────────────────────────────────────┤
│                   GAME CLIENT (JS)                      │
│        app/game.html · isometric · Canvas/WebGPU        │
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
| `bifrost-witness` | Witness Quorum (BLAKE3 consensus) |
| `bifrost-physics` | Deterministic Physics (f64, BTreeMap, no SystemTime) |
| `bifrost-wac` | World Asset Compiler + Canonical Biome Registry |
| `bifrost-aigm` | AI Game Master — NPC AI, quests, story, LLM dialogue |
| `bifrost-run` | World Run System — epochs, win conditions, meta progression |
| `bifrost-synthesis` | Synthesis AI Civilization |
| `bifrost-safe-city` | Safe City + Auction House + Zone Control |
| `bifrost-server` | HTTP REST — 40+ routes |

### Nexus — Voxel World

| Crate | Purpose |
|---|---|
| `nexus-voxel-kernel` | Greedy meshing, NavMesh (A*), fBm noise, biome gen, WAC RT |

### Client Runtime (nova-* crates)

| Crate | Purpose |
|---|---|
| `nova-core` | ECS World, Transform3D, SceneGraph, Timer |
| `nova-render` | WebGPU — GpuVoxelVertex, Camera3D, WGSL shaders |
| `nova-anim` | VoxelSkeleton, AnimClip (slerp), AnimFSM |
| `nova-input` | KeyCode/MouseButton → ActionId, InputMap |

---

## Quick Start

```bash
cargo run -p bifrost-server    # server on :8080
open http://localhost:8080     # game client
```

---

## Docs

```
docs/
├── engine/
│   ├── architecture.md        system design, authority model
│   ├── bifrost-protocol.md    VIS opcodes, epochs, lockstep, witness
│   ├── wac.md                 World Asset Compiler + biome registry
│   └── client-runtime.md      ECS, WebGPU, AnimFSM, Input (nova-* crates)
├── game/
│   ├── world.md               run system, zones, economy, biome evolution
│   ├── factions.md            Synthesis AI civilization
│   ├── players.md             classes, clone system, meta progression
│   ├── npcs.md                AI GM layers, village NPCs, dialogue
│   ├── monsters.md            types, stats, drops, boss mechanics
│   ├── quests.md              quest chains, states, server authority
│   └── skills.md              skill trees (Warrior/Mage/Rogue), world skills
├── api/
│   └── usage.md               all 40+ HTTP routes with curl examples
├── ops/
│   └── docker.md              Docker / BuildKit troubleshooting
└── planning/
    └── roadmap.md             crate status, open PRs, phase roadmap
```
