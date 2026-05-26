# Bifrost Layer — Usage Guide

> **DELPHOS decides truth. The players compute reality.**

---

## Quick Start

### Option A — Rust server only (no Docker)

```bash
# Clone and build
git clone https://github.com/soniaschuh88-ship-it/mmo
cd mmo

# Start bifrost-server (port 8080)
cargo run -p bifrost-server

# In another terminal — run the full pipeline demo
curl -s -X POST http://localhost:8080/demo | jq .
```

### Option B — Full stack with Docker

```bash
# Build the container (Rust + Node gateway)
docker build -t bifrost .

# Run
docker run -p 8080:8080 bifrost

# Open the demo UI
open http://localhost:8080
```

---

## API Reference

### Meta

```bash
# API info + endpoint list
curl http://localhost:8080/

# Health check
curl http://localhost:8080/health

# Current simulation state
curl http://localhost:8080/state
```

---

### Full Pipeline Demo (no setup needed)

```bash
curl -s -X POST http://localhost:8080/demo | jq .
```

**Response:**
```json
{
  "peers": 3,
  "instructions": 2,
  "voxels_before": 0,
  "voxels_after": 92,
  "state_hash": "a3f4b2...",
  "consensus": "accepted",
  "tick_advanced": true,
  "new_tick": 1,
  "steps": [
    "registered 3 peers (1 authority + 2 witnesses)",
    "built VoxelProgram: 2 instructions (FILL_BOX + SIM_EXPLOSION)",
    "physics executed: 0 voxels before → 92 after, state_hash=a3f4b2...",
    "all 3 core peers submitted witness votes",
    "witness consensus: accepted",
    "tick advanced: true → new_tick=1"
  ]
}
```

---

### Peer Management

```bash
# Register a peer (peer_id = 64-char hex = 32-byte Ed25519 pubkey)
PEER1="0101010101010101010101010101010101010101010101010101010101010101"
PEER2="0202020202020202020202020202020202020202020202020202020202020202"
PEER3="0303030303030303030303030303030303030303030303030303030303030303"

curl -X POST http://localhost:8080/peers \
     -H 'Content-Type: application/json' \
     -d "{\"peer_id\":\"$PEER1\"}"

# Evict a peer
curl -X DELETE "http://localhost:8080/peers/$PEER1"
```

---

### Tick Management

```bash
# Current tick + lagging peers
curl http://localhost:8080/tick

# Submit a VoxelProgram for tick 0
curl -X POST http://localhost:8080/tick/input \
     -H 'Content-Type: application/json' \
     -d "{
  \"peer_id\": \"$PEER1\",
  \"tick\": 0,
  \"instructions\": [
    {
      \"epoch\": 0,
      \"payload\": {
        \"op\": \"SimExplosion\",
        \"center\": {\"x\": 0, \"y\": 0, \"z\": 0},
        \"radius\": 8,
        \"force\": 1000,
        \"result_material\": 0
      }
    }
  ]
}"

# Acknowledge tick completion
curl -X POST http://localhost:8080/tick/ack \
     -H 'Content-Type: application/json' \
     -d "{\"peer_id\":\"$PEER1\",\"tick\":0}"

# Try to advance to next tick (requires all peers acked)
curl -X POST http://localhost:8080/tick/advance
```

---

### World Operations

```bash
# Get world state
curl http://localhost:8080/world/state

# Execute a single instruction immediately
curl -X POST http://localhost:8080/world/instruction \
     -H 'Content-Type: application/json' \
     -d '{
  "epoch": 0,
  "payload": {
    "op": "FillBox",
    "min": {"x": -10, "y": -10, "z": -10},
    "max": {"x": 10, "y": 10, "z": 10},
    "material": 1
  }
}'
```

---

### Witness Consensus

```bash
HASH="aabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccdd"

# Configure witness quorum
curl -X POST http://localhost:8080/witness/setup \
     -H 'Content-Type: application/json' \
     -d "{
  \"authority\": \"$PEER1\",
  \"witnesses\": [\"$PEER2\", \"$PEER3\"]
}"

# Submit witness votes (all agree = ACCEPTED)
for PEER in $PEER1 $PEER2 $PEER3; do
  ROLE="witness"
  [ "$PEER" = "$PEER1" ] && ROLE="authority"
  curl -X POST http://localhost:8080/witness/vote \
       -H 'Content-Type: application/json' \
       -d "{\"peer_id\":\"$PEER\",\"tick\":0,\"tick_hash\":\"$HASH\",\"role\":\"$ROLE\"}"
done

# Check consensus
curl http://localhost:8080/witness/consensus/0
```

---

## VIS Opcode Reference

| Opcode | `"op"` field | Required fields |
|---|---|---|
| `SET_VOXEL` | `"SetVoxel"` | `position: {x,y,z}`, `material: u8` |
| `FILL_BOX` | `"FillBox"` | `min: {x,y,z}`, `max: {x,y,z}`, `material: u8` |
| `SPHERE_CUT` | `"SphereCut"` | `center: {x,y,z}`, `radius: u32`, `material: u8` |
| `MARCH_MATERIAL` | `"MarchMaterial"` | `origin: {x,y,z}`, `direction: [i8;3]`, `steps: u32`, `material: u8` |
| `DAMAGE_FIELD` | `"DamageField"` | `center: {x,y,z}`, `radius: u32`, `damage: u16` |
| `SIM_WATER` | `"SimWater"` | `origin: {x,y,z}`, `volume: u32`, `pressure: u16` |
| `SIM_FIRE` | `"SimFire"` | `origin: {x,y,z}`, `intensity: u16`, `fuel: u32` |
| `SIM_DEBRIS` | `"SimDebris"` | `origin: {x,y,z}`, `count: u32`, `impulse: u16` |
| `SIM_EXPLOSION` | `"SimExplosion"` | `center: {x,y,z}`, `radius: u32`, `force: u32`, `result_material: u8` |

---

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `8080` | Port for bifrost-server or Node gateway |
| `BIFROST_SERVER_URL` | `http://localhost:8080` | URL of bifrost-server (used by Node gateway) |
| `RUST_LOG` | `info` | Log level (`debug` for detailed output) |
