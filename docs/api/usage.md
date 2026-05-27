# bKG — Bifrost MMO · Usage Guide

> **bKG** · *best known Garbage* · DELPHOS decides truth. The players compute reality.

---

## Quick Start

### Option A — Rust server only

```bash
cargo run -p bifrost-server
# Server listens on http://localhost:8080
```

### Option B — Full stack with Docker

```bash
docker build -t nova-mmo .
docker run -p 8080:8080 nova-mmo
open http://localhost:8080
```

---

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `8080` | Port for bifrost-server or Node gateway |
| `BIFROST_SERVER_URL` | `http://localhost:8080` | URL of bifrost-server (Node gateway) |
| `RUST_LOG` | `info` | Log level (`debug` for verbose output) |
| `NVIDIA_API_KEY` | — | NVIDIA NIM API key (nvidia-nim feature) |
| `NVIDIA_NIM_BASE_URL` | `https://integrate.api.nvidia.com/v1` | NIM endpoint |
| `NVIDIA_NIM_MODEL` | `meta/llama-3.3-70b-instruct` | NIM model name |

---

## API Reference

### Meta

```bash
curl http://localhost:8080/            # API info + endpoint list
curl http://localhost:8080/health      # Health check
curl http://localhost:8080/state       # Simulation state summary
```

### Full Pipeline Demo

```bash
curl -s -X POST http://localhost:8080/demo | jq .
```

---

### Peer Management

```bash
PEER1="0101010101010101010101010101010101010101010101010101010101010101"

curl -X POST http://localhost:8080/peers \
     -H 'Content-Type: application/json' \
     -d "{\"peer_id\":\"$PEER1\"}"

curl -X DELETE "http://localhost:8080/peers/$PEER1"
```

---

### Tick

```bash
curl http://localhost:8080/tick                         # current tick
curl -X POST http://localhost:8080/tick/input  -H 'Content-Type: application/json' \
     -d "{\"peer_id\":\"$PEER1\",\"tick\":0,\"instructions\":[]}"
curl -X POST http://localhost:8080/tick/ack    -H 'Content-Type: application/json' \
     -d "{\"peer_id\":\"$PEER1\",\"tick\":0}"
curl -X POST http://localhost:8080/tick/advance
```

---

### World

```bash
curl http://localhost:8080/world/state

curl -X POST http://localhost:8080/world/instruction \
     -H 'Content-Type: application/json' \
     -d '{"epoch":0,"payload":{"op":"FillBox","min":{"x":-5,"y":-5,"z":-5},"max":{"x":5,"y":5,"z":5},"material":1}}'
```

---

### Witness Consensus

```bash
HASH="aabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccdd"

curl -X POST http://localhost:8080/witness/setup \
     -H 'Content-Type: application/json' \
     -d "{\"authority\":\"$PEER1\",\"witnesses\":[\"$PEER2\",\"$PEER3\"]}"

curl -X POST http://localhost:8080/witness/vote \
     -H 'Content-Type: application/json' \
     -d "{\"peer_id\":\"$PEER1\",\"tick\":0,\"tick_hash\":\"$HASH\",\"role\":\"authority\"}"

curl http://localhost:8080/witness/consensus/0
```

---

### WAC — World Asset Compiler

```bash
# Compile a biome blueprint
curl -X POST http://localhost:8080/wac/compile \
     -H 'Content-Type: application/json' \
     -d '{
       "id": "00000000-0000-0000-0000-000000000001",
       "asset_type": "biome_definition",
       "natural_language_spec": "dense crimson crystal forest with nocturnal glow",
       "constraints": ["no floating tiles"],
       "seed": 42
     }'

curl http://localhost:8080/wac/cache/stats
curl -X POST http://localhost:8080/wac/director/tick \
     -H 'Content-Type: application/json' -d '{}'
```

---

### Nexus Voxel Kernel

```bash
# Generate a chunk from a WAC document
curl -X POST http://localhost:8080/nexus/wac \
     -H 'Content-Type: application/json' \
     -d '{"type":"chunk","pos":{"x":0,"y":0,"z":0},"biome":"crimson_forest"}'

curl http://localhost:8080/nexus/biomes           # list all 14 canonical biomes
curl http://localhost:8080/nexus/chunk/0/0/0      # inspect cached chunk
curl http://localhost:8080/nexus/world            # world stats
curl -X POST http://localhost:8080/nexus/demo     # generate 3 demo chunks
```

---

### World Run System

```bash
# Start a new run epoch
curl -X POST http://localhost:8080/run \
     -H 'Content-Type: application/json' \
     -d '{"label":"Season 1","end_condition":{"type":"first_to_control_zones","zones":3}}'

curl http://localhost:8080/run/current            # active run state
curl -X POST http://localhost:8080/run/tick \
     -H 'Content-Type: application/json' \
     -d '{"tick":100,"zone_controls":{}}'         # evaluate win conditions
curl -X POST http://localhost:8080/run/end        # force-end active run
curl http://localhost:8080/run/history            # all runs
```

---

### Synthesis AI

```bash
curl -X POST http://localhost:8080/synthesis/init \
     -H 'Content-Type: application/json' \
     -d '{"faction_id":"synthesis-alpha"}'

curl http://localhost:8080/synthesis/faction       # faction state
curl http://localhost:8080/synthesis/agents        # agent list

curl -X POST http://localhost:8080/synthesis/tick \
     -H 'Content-Type: application/json' \
     -d '{"tick":1,"zone_resources":{"outer-east":0.7},"player_economy_fraction":0.4}'
```

---

### Safe City + Economy

```bash
curl http://localhost:8080/safe-city              # city state
curl http://localhost:8080/safe-city/auction       # active listings
curl http://localhost:8080/safe-city/zones         # all zones
curl http://localhost:8080/safe-city/zones/outer-east

# Post a listing
curl -X POST http://localhost:8080/safe-city/auction/list \
     -H 'Content-Type: application/json' \
     -d '{"item_id":"crystal_shard","quantity":10,"price_per_unit":5,"seller_id":"player-1"}'

# Buy a listing
curl -X POST http://localhost:8080/safe-city/auction/buy \
     -H 'Content-Type: application/json' \
     -d '{"listing_id":"<id>","buyer_id":"player-2","quantity":3}'

# Apply faction influence
curl -X POST http://localhost:8080/safe-city/zones/outer-east/influence \
     -H 'Content-Type: application/json' \
     -d '{"faction_id":"synthesis-alpha","delta":0.1}'
```

---

## VIS Opcode Reference

| Opcode | `"op"` field | Required fields |
|---|---|---|
| SET_VOXEL | `"SetVoxel"` | `position:{x,y,z}`, `material:u8` |
| FILL_BOX | `"FillBox"` | `min:{x,y,z}`, `max:{x,y,z}`, `material:u8` |
| SPHERE_CUT | `"SphereCut"` | `center:{x,y,z}`, `radius:u32`, `material:u8` |
| MARCH_MATERIAL | `"MarchMaterial"` | `origin`, `direction:[i8;3]`, `steps:u32`, `material:u8` |
| DAMAGE_FIELD | `"DamageField"` | `center:{x,y,z}`, `radius:u32`, `damage:u16` |
| SIM_WATER | `"SimWater"` | `origin:{x,y,z}`, `volume:u32`, `pressure:u16` |
| SIM_FIRE | `"SimFire"` | `origin:{x,y,z}`, `intensity:u16`, `fuel:u32` |
| SIM_DEBRIS | `"SimDebris"` | `origin:{x,y,z}`, `count:u32`, `impulse:u16` |
| SIM_EXPLOSION | `"SimExplosion"` | `center:{x,y,z}`, `radius:u32`, `force:u32`, `result_material:u8` |

---

## Docker

See [`docs/DOCKER.md`](DOCKER.md) for BuildKit/Bake troubleshooting.

```bash
DOCKER_BUILDKIT=0 docker compose up --build
```
