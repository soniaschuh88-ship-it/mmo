# Bifrost Protocol Specification

> Bifrost connects DELPHOS (truth authority) to the MMO Fabric (distributed compute).

---

## 1. Voxel Instruction Set (VIS)

### Opcodes

| Opcode | Code | Parameters | Description |
|---|---|---|---|
| `SET_VOXEL` | 0x01 | x,y,z, material | Set single voxel |
| `FILL_BOX` | 0x02 | min, max, material | Fill AABB with material |
| `SPHERE_CUT` | 0x03 | center, radius, material | Spherical excavation |
| `MARCH_MATERIAL` | 0x04 | origin, direction, steps, material | Marching cubes fill |
| `DAMAGE_FIELD` | 0x05 | center, radius, damage | Apply damage to voxels |
| `SIM_WATER` | 0x06 | origin, volume, pressure | Fluid simulation tick |
| `SIM_FIRE` | 0x07 | origin, intensity, fuel | Fire propagation tick |
| `SIM_DEBRIS` | 0x08 | origin, count, impulse | Debris scatter |
| `SIM_EXPLOSION` | 0x09 | center, radius, force | Explosion with physics |

### VoxelProgram Hash

A `VoxelProgram` is a sequence of instructions, hashed with BLAKE3:

```
program_hash = BLAKE3(concat(instruction_hashes))
instruction_hash = BLAKE3(opcode || epoch || payload_bytes)
```

All peers must produce identical `instruction_hash` for the same instruction.

---

## 2. Chunk Authority Epochs

### Chunk Coordinate System

Chunks are identified by `(x, y, z, lod)` where:
- `x,y,z` are chunk grid coordinates (each chunk = 64×64×64 voxels)
- `lod` is the level-of-detail tier (0 = full, 1 = half, etc.)

### Authority Rotation

Every `epoch_duration_ticks` (default: 1000 ticks), the chunk authority rotates:

```
new_authority = peer_pool[(epoch_number % len(peer_pool))]
```

This prevents any single peer from controlling a chunk indefinitely.

### Epoch Boundary

An `EpochBoundary` is a signed checkpoint:

```
EpochBoundary {
    chunk_id,
    epoch_number,
    outgoing_authority,
    incoming_authority,
    final_state_hash,   // BLAKE3 of chunk state at epoch end
    signature,          // Ed25519 by outgoing_authority
}
```

The `final_state_hash` is the replay anchor for new authorities.

---

## 3. Lockstep Tick Protocol

### Tick Advance Rule

**Tick N+1 starts only when all registered peers have acknowledged Tick N.**

```
advance_tick(N → N+1) requires:
  ∀ peer ∈ registered_peers: peer.last_ack >= N
```

Slow peers cause backpressure. Unresponsive peers are evicted after timeout.

### InputBuffer

Each peer submits their `VoxelProgram` for a tick before the barrier is released:

```
InputBuffer[tick] = {
    peer_id → VoxelProgram
}
```

The merged program for tick N is the deterministic sort of all peer programs.

---

## 4. Witness Quorum Protocol

### Roles

| Role | Count | Responsibility |
|---|---|---|
| Authority | 1 | Executes tick, produces reference hash |
| Witness | 2 | Independent execution, vote on hash |
| Advisory | N | Soft votes, trust signal (non-binding) |

### Consensus Rules

```
ACCEPTED:   authority_hash == witness_1_hash == witness_2_hash
CONTESTED:  any mismatch among authority + witnesses
PENDING:    waiting for votes
```

### Contested Resolution

1. Record mismatched peer IDs
2. Identify `replay_from_tick` (last accepted tick)
3. Promote a new witness from advisory pool
4. Reduce trust score of mismatching peer
5. Replay from `replay_from_tick` with new quorum

---

## 5. Deterministic Physics Contract

All peers MUST produce identical physics output for identical inputs.

### Guarantees

- No `HashMap` — use `BTreeMap` for stable iteration order
- No `SystemTime` — use tick number as time reference
- No `f32` accumulation — use `f64` with deterministic rounding contract
- No OS-specific behavior — pure computation only
- WASM-compilable — same binary on browser, desktop, mobile, edge

### WASM Interface (Future)

```
fn execute_physics_tick(
    world_state: &[u8],   // CBOR-encoded PhysicsWorld
    instructions: &[u8],  // CBOR-encoded Vec<VoxelInstruction>
) -> Vec<u8>              // CBOR-encoded (new_world_state, state_hash)
```

This interface allows the browser to run **identical physics** to the server,
enabling witness verification without server involvement.
