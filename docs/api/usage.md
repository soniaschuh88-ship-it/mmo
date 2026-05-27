# bKG — Bifrost MMO · Usage Guide

> *bKG · best known Garbage* · DELPHOS decides truth. The players compute reality.

---

## Quick Start

```bash
cargo run -p bifrost-server        # Rust server on :8080
# or
docker build -t bkg-bifrost . && docker run -p 8080:8080 bkg-bifrost
open http://localhost:8080
```

---

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `8080` | Server port |
| `BIFROST_SERVER_URL` | `http://localhost:8080` | Node gateway target |
| `RUST_LOG` | `info` | Log level |
| `NVIDIA_API_KEY` | — | NIM API key (nvidia-nim feature) |
| `NVIDIA_NIM_BASE_URL` | `https://integrate.api.nvidia.com/v1` | NIM endpoint |
| `NVIDIA_NIM_MODEL` | `meta/llama-3.3-70b-instruct` | NIM model |

---

## API Reference

### Meta

```bash
curl http://localhost:8080/            # endpoint list
curl http://localhost:8080/health
curl http://localhost:8080/state
curl -X POST http://localhost:8080/demo | jq .
```

### Peers

```bash
P="0101010101010101010101010101010101010101010101010101010101010101"
curl -X POST http://localhost:8080/peers -H 'Content-Type: application/json' -d "{\"peer_id\":\"$P\"}"
curl -X DELETE "http://localhost:8080/peers/$P"
```

### Tick

```bash
curl http://localhost:8080/tick
curl -X POST http://localhost:8080/tick/input  -H 'Content-Type: application/json' \
     -d "{\"peer_id\":\"$P\",\"tick\":0,\"instructions\":[]}"
curl -X POST http://localhost:8080/tick/ack    -H 'Content-Type: application/json' \
     -d "{\"peer_id\":\"$P\",\"tick\":0}"
curl -X POST http://localhost:8080/tick/advance
```

### World

```bash
curl http://localhost:8080/world/state
curl -X POST http://localhost:8080/world/instruction -H 'Content-Type: application/json' \
     -d '{"epoch":0,"payload":{"op":"FillBox","min":{"x":-5,"y":-5,"z":-5},"max":{"x":5,"y":5,"z":5},"material":1}}'
```

### Witness

```bash
H="aabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccdd"
curl -X POST http://localhost:8080/witness/setup -H 'Content-Type: application/json' \
     -d "{\"authority\":\"$P1\",\"witnesses\":[\"$P2\",\"$P3\"]}"
curl -X POST http://localhost:8080/witness/vote -H 'Content-Type: application/json' \
     -d "{\"peer_id\":\"$P1\",\"tick\":0,\"tick_hash\":\"$H\",\"role\":\"authority\"}"
curl http://localhost:8080/witness/consensus/0
```

### WAC

```bash
curl -X POST http://localhost:8080/wac/compile -H 'Content-Type: application/json' \
     -d '{"id":"00000000-0000-0000-0000-000000000001","asset_type":"biome_definition",
          "natural_language_spec":"dense crimson crystal forest","constraints":[],"seed":42}'
curl http://localhost:8080/wac/cache/stats
curl -X POST http://localhost:8080/wac/director/tick -H 'Content-Type: application/json' -d '{}'
```

### Nexus Voxel Kernel

```bash
curl -X POST http://localhost:8080/nexus/wac -H 'Content-Type: application/json' \
     -d '{"type":"chunk","pos":{"x":0,"y":0,"z":0},"biome":"crimson_forest"}'
curl http://localhost:8080/nexus/biomes
curl http://localhost:8080/nexus/chunk/0/0/0
curl http://localhost:8080/nexus/world
curl -X POST http://localhost:8080/nexus/demo
```

### World Run System

```bash
curl -X POST http://localhost:8080/run -H 'Content-Type: application/json' \
     -d '{"label":"Season 1","end_condition":{"type":"first_to_control_zones","zones":3}}'
curl http://localhost:8080/run/current
curl -X POST http://localhost:8080/run/tick  -H 'Content-Type: application/json' \
     -d '{"tick":100,"zone_controls":{}}'
curl -X POST http://localhost:8080/run/end
curl http://localhost:8080/run/history
```

### Synthesis AI

```bash
curl -X POST http://localhost:8080/synthesis/init -H 'Content-Type: application/json' \
     -d '{"faction_id":"synthesis-alpha"}'
curl http://localhost:8080/synthesis/faction
curl http://localhost:8080/synthesis/agents
curl -X POST http://localhost:8080/synthesis/tick -H 'Content-Type: application/json' \
     -d '{"tick":1,"zone_resources":{"outer-east":0.7},"player_economy_fraction":0.4}'
```

### Safe City + Economy

```bash
curl http://localhost:8080/safe-city
curl http://localhost:8080/safe-city/auction
curl http://localhost:8080/safe-city/zones
curl http://localhost:8080/safe-city/zones/outer-east

curl -X POST http://localhost:8080/safe-city/auction/list -H 'Content-Type: application/json' \
     -d '{"item_id":"crystal_shard","quantity":10,"price_per_unit":5,"seller_id":"p1"}'
curl -X POST http://localhost:8080/safe-city/auction/buy -H 'Content-Type: application/json' \
     -d '{"listing_id":"<id>","buyer_id":"p2","quantity":3}'
curl -X POST http://localhost:8080/safe-city/zones/outer-east/influence \
     -H 'Content-Type: application/json' \
     -d '{"faction_id":"synthesis-alpha","delta":0.1}'
```

---

## VIS Opcode Reference

| Opcode | `"op"` | Required fields |
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

## See Also

- [`engine/bifrost-protocol.md`](../engine/bifrost-protocol.md) — Full protocol spec
- [`ops/docker.md`](../ops/docker.md) — Docker troubleshooting
