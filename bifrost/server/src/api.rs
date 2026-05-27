//! HTTP route handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use bifrost_chunk::PeerId;
use bifrost_lockstep::{LockstepScheduler, LockstepTick};
use bifrost_physics::{PhysicsExecutor, PhysicsWorld};
use bifrost_vis::{InstructionPayload, VoxelInstruction, VoxelProgram};
use bifrost_witness::{
    ConsensusResult, PeerRole, TickHash, WitnessExecutor, WitnessVote,
};
use bifrost_wac::{
    AssetBlueprint, PressureGraph,
    compile, validate,
    cache::semantic_hash_with_seed,
};

use crate::models::*;
use crate::state::SharedState;

// ─── Helper ───────────────────────────────────────────────────────────────────

/// Return a 400 JSON error response.
macro_rules! bad {
    ($msg:expr) => {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResp::new($msg)),
        ).into_response()
    };
}

// ─── Root + Health ────────────────────────────────────────────────────────────

pub async fn root() -> Json<ApiInfo> {
    Json(ApiInfo {
        name:    "bifrost-server",
        version: "0.1.0",
        endpoints: vec![
            "GET  /",
            "GET  /health",
            "POST /demo",
            "GET  /state",
            "POST /peers",
            "DELETE /peers/:peer_id",
            "GET  /tick",
            "POST /tick/input",
            "POST /tick/ack",
            "POST /tick/advance",
            "POST /world/instruction",
            "GET  /world/state",
            "POST /witness/setup",
            "POST /witness/vote",
            "GET  /witness/consensus/:tick",
        ],
    })
}

pub async fn health(State(shared): State<SharedState>) -> Json<HealthResp> {
    let s = shared.lock().await;
    Json(HealthResp {
        status: "ok",
        tick:   s.world.tick(),
        voxels: s.world.voxel_count(),
        peers:  s.peers.len(),
    })
}

pub async fn get_state(State(shared): State<SharedState>) -> Json<StateResp> {
    let s = shared.lock().await;
    Json(StateResp {
        tick:        s.world.tick(),
        state_hash:  hex::encode(s.world.state_hash()),
        voxel_count: s.world.voxel_count(),
        peer_count:  s.peers.len(),
    })
}

// ─── Demo ─────────────────────────────────────────────────────────────────────

/// Run a complete isolated Bifrost pipeline:
/// fill + explode → physics → witness consensus → tick advance.
pub async fn demo() -> Json<DemoResult> {
    use bifrost_vis::{
        FillBoxPayload, SimExplosionPayload, VoxelCoord,
    };
    use bifrost_physics::{MAT_AIR, MAT_STONE};

    let mut steps: Vec<String> = Vec::new();

    // 1. Register 3 peers
    let authority = PeerId([0x01u8; 32]);
    let witness1  = PeerId([0x02u8; 32]);
    let witness2  = PeerId([0x03u8; 32]);
    let mut sched = LockstepScheduler::new(50);
    sched.register_peer(authority);
    sched.register_peer(witness1);
    sched.register_peer(witness2);
    let mut exec = WitnessExecutor::new(authority, [witness1, witness2], vec![]);
    steps.push("registered 3 peers (1 authority + 2 witnesses)".into());

    // 2. Build a VoxelProgram: fill a 5×5×5 box with stone, then explode the center
    let mut program = VoxelProgram::new();
    program.push(0, InstructionPayload::FillBox(FillBoxPayload {
        min:      VoxelCoord::new(-2, -2, -2),
        max:      VoxelCoord::new(2, 2, 2),
        material: MAT_STONE,
    })).unwrap();
    program.push(0, InstructionPayload::SimExplosion(SimExplosionPayload {
        center:          VoxelCoord::new(0, 0, 0),
        radius:          2,
        force:           800,
        result_material: MAT_AIR,
    })).unwrap();
    let instr_count = program.len();
    steps.push(format!("built VoxelProgram: {} instructions (FILL_BOX + SIM_EXPLOSION)", instr_count));

    // 3. Execute physics
    let mut world = PhysicsWorld::new();
    let voxels_before = world.voxel_count();
    let result = PhysicsExecutor::execute_program(&mut world, &program);
    let voxels_after = world.voxel_count();
    steps.push(format!(
        "physics executed: {} voxels before → {} after, state_hash={}",
        voxels_before, voxels_after, hex::encode(&result.state_hash[..4])
    ));

    // 4. Submit identical witness votes (simulates full agreement)
    let tick      = LockstepTick::from_legacy(0);
    let tick_hash = TickHash::from_bytes(result.state_hash);
    exec.submit_vote(WitnessVote::unsigned(authority, tick, tick_hash, PeerRole::Authority)).unwrap();
    exec.submit_vote(WitnessVote::unsigned(witness1,  tick, tick_hash, PeerRole::Witness)).unwrap();
    exec.submit_vote(WitnessVote::unsigned(witness2,  tick, tick_hash, PeerRole::Witness)).unwrap();
    steps.push("all 3 core peers submitted witness votes".into());

    // 5. Evaluate consensus
    let consensus = exec.evaluate_consensus(tick);
    let consensus_str = match &consensus {
        ConsensusResult::Accepted { .. } => "accepted",
        ConsensusResult::Contested { .. } => "contested",
        ConsensusResult::Pending { .. } => "pending",
    };
    steps.push(format!("witness consensus: {}", consensus_str));

    // 6. Advance tick
    sched.record_ack(authority, tick);
    sched.record_ack(witness1, tick);
    sched.record_ack(witness2, tick);
    let advance = sched.try_advance();
    let tick_advanced = advance.is_some();
    steps.push(format!("tick advanced: {} → new_tick={}", tick_advanced, sched.current_tick().local_seq()));

    Json(DemoResult {
        peers: 3,
        instructions: instr_count,
        voxels_before,
        voxels_after,
        state_hash:    hex::encode(result.state_hash),
        consensus:     consensus_str,
        tick_advanced,
        new_tick:      sched.current_tick().local_seq(),
        steps,
    })
}

// ─── Peers ────────────────────────────────────────────────────────────────────

pub async fn register_peer(
    State(shared): State<SharedState>,
    Json(req): Json<RegisterPeerReq>,
) -> impl IntoResponse {
    let peer = match crate::state::SimState::parse_peer_id(&req.peer_id) {
        Ok(p)  => p,
        Err(e) => return bad!(e),
    };
    let mut s = shared.lock().await;
    if !s.peers.contains(&peer) {
        s.peers.push(peer);
        s.scheduler.register_peer(peer);
    }
    (StatusCode::CREATED, Json(PeerResp {
        peer_id:    req.peer_id,
        action:     "registered",
        peer_count: s.peers.len(),
    })).into_response()
}

pub async fn evict_peer(
    State(shared): State<SharedState>,
    Path(peer_id_hex): Path<String>,
) -> impl IntoResponse {
    let peer = match crate::state::SimState::parse_peer_id(&peer_id_hex) {
        Ok(p)  => p,
        Err(e) => return bad!(e),
    };
    let mut s = shared.lock().await;
    s.peers.retain(|p| p != &peer);
    s.scheduler.evict_peer(&peer);
    Json(PeerResp {
        peer_id:    peer_id_hex,
        action:     "evicted",
        peer_count: s.peers.len(),
    }).into_response()
}

// ─── Tick ─────────────────────────────────────────────────────────────────────

pub async fn get_tick(State(shared): State<SharedState>) -> Json<TickResp> {
    let s = shared.lock().await;
    let lagging = s.scheduler
        .lagging_peers()
        .iter()
        .map(|p| hex::encode(p.as_bytes()))
        .collect();
    Json(TickResp {
        current_tick:  s.scheduler.current_tick().local_seq(),
        lagging_peers: lagging,
    })
}

pub async fn submit_input(
    State(shared): State<SharedState>,
    Json(req): Json<SubmitInputReq>,
) -> impl IntoResponse {
    let peer = match crate::state::SimState::parse_peer_id(&req.peer_id) {
        Ok(p)  => p,
        Err(e) => return bad!(e),
    };
    let tick = LockstepTick::from_legacy(req.tick);

    // Build VoxelProgram from raw instructions
    let mut program = VoxelProgram::new();
    for raw in &req.instructions {
        let payload: InstructionPayload = match serde_json::from_value(raw.payload.clone()) {
            Ok(p)  => p,
            Err(e) => return bad!(format!("invalid payload: {e}")),
        };
        if let Err(e) = program.push(raw.epoch, payload) {
            return bad!(format!("instruction error: {e}"));
        }
    }

    let instr_count  = program.len();
    let program_hash = hex::encode(program.program_hash);

    let mut s = shared.lock().await;
    if let Err(e) = s.scheduler.submit_input(peer, tick, program) {
        return bad!(format!("{e}"));
    }

    Json(InputResp {
        accepted:     true,
        peer_id:      req.peer_id,
        tick:         req.tick,
        program_hash,
        instr_count,
    }).into_response()
}

pub async fn ack_tick(
    State(shared): State<SharedState>,
    Json(req): Json<AckReq>,
) -> impl IntoResponse {
    let peer = match crate::state::SimState::parse_peer_id(&req.peer_id) {
        Ok(p)  => p,
        Err(e) => return bad!(e),
    };
    let mut s = shared.lock().await;
    s.scheduler.record_ack(peer, LockstepTick::from_legacy(req.tick));
    Json(json!({ "acked": true, "peer_id": req.peer_id, "tick": req.tick })).into_response()
}

pub async fn advance_tick(State(shared): State<SharedState>) -> impl IntoResponse {
    let mut s = shared.lock().await;

    // Collect all pending inputs for current tick and execute physics
    let current = s.scheduler.current_tick();
    let advance = s.scheduler.try_advance();

    match advance {
        None => {
            (StatusCode::ACCEPTED, Json(AdvanceResp {
                advanced:             false,
                current_tick:         current.local_seq(),
                completed_tick:       None,
                state_hash:           hex::encode(s.world.state_hash()),
                instructions_executed: 0,
            })).into_response()
        }
        Some(adv) => {
            // Execute all programs from this tick
            let mut instr_total = 0usize;
            for (_peer, program) in &adv.inputs {
                let result = PhysicsExecutor::execute_program(&mut s.world, program);
                instr_total += result.instr_count;
                // NOTE: execute_program advances the world tick; for multi-peer
                // inputs we snapshot and merge in a future revision.
            }
            let state_hash = hex::encode(s.world.state_hash());
            Json(AdvanceResp {
                advanced:             true,
                current_tick:         s.scheduler.current_tick().local_seq(),
                completed_tick:       Some(adv.completed_tick.local_seq()),
                state_hash,
                instructions_executed: instr_total,
            }).into_response()
        }
    }
}

// ─── World ────────────────────────────────────────────────────────────────────

pub async fn world_state(State(shared): State<SharedState>) -> Json<WorldResp> {
    let s = shared.lock().await;
    Json(WorldResp {
        tick:        s.world.tick(),
        state_hash:  hex::encode(s.world.state_hash()),
        voxel_count: s.world.voxel_count(),
    })
}

pub async fn execute_instruction(
    State(shared): State<SharedState>,
    Json(req): Json<ExecuteInstructionReq>,
) -> impl IntoResponse {
    let payload: InstructionPayload = match serde_json::from_value(req.payload) {
        Ok(p)  => p,
        Err(e) => return bad!(format!("invalid payload: {e}")),
    };
    let instr = match VoxelInstruction::new(req.epoch, payload) {
        Ok(i)  => i,
        Err(e) => return bad!(format!("instruction error: {e}")),
    };
    let mut s = shared.lock().await;
    PhysicsExecutor::execute_instruction(&mut s.world, &instr);
    Json(WorldResp {
        tick:        s.world.tick(),
        state_hash:  hex::encode(s.world.state_hash()),
        voxel_count: s.world.voxel_count(),
    }).into_response()
}

// ─── Witness ──────────────────────────────────────────────────────────────────

pub async fn setup_witness(
    State(shared): State<SharedState>,
    Json(req): Json<SetupWitnessReq>,
) -> impl IntoResponse {
    let authority = match crate::state::SimState::parse_peer_id(&req.authority) {
        Ok(p)  => p,
        Err(e) => return bad!(e),
    };
    let w0 = match crate::state::SimState::parse_peer_id(&req.witnesses[0]) {
        Ok(p)  => p,
        Err(e) => return bad!(e),
    };
    let w1 = match crate::state::SimState::parse_peer_id(&req.witnesses[1]) {
        Ok(p)  => p,
        Err(e) => return bad!(e),
    };
    let mut s = shared.lock().await;
    if let Some(ex) = &mut s.witness {
        ex.set_quorum(authority, [w0, w1]);
    } else {
        s.witness = Some(WitnessExecutor::new(authority, [w0, w1], vec![]));
    }
    Json(WitnessSetupResp {
        authority: req.authority,
        witnesses: [req.witnesses[0].clone(), req.witnesses[1].clone()],
    }).into_response()
}

pub async fn submit_witness_vote(
    State(shared): State<SharedState>,
    Json(req): Json<WitnessVoteReq>,
) -> impl IntoResponse {
    let peer = match crate::state::SimState::parse_peer_id(&req.peer_id) {
        Ok(p)  => p,
        Err(e) => return bad!(e),
    };
    let hash_bytes = match hex::decode(&req.tick_hash) {
        Ok(b)  => b,
        Err(_) => return bad!("tick_hash must be 64-char hex"),
    };
    let hash_arr: [u8; 32] = match hash_bytes.try_into() {
        Ok(a)  => a,
        Err(_) => return bad!("tick_hash must be 32 bytes (64 hex chars)"),
    };
    let role = match req.role.as_str() {
        "authority" => PeerRole::Authority,
        "witness"   => PeerRole::Witness,
        "advisory"  => PeerRole::Advisory,
        other       => return bad!(format!("unknown role: {other}")),
    };
    let vote = WitnessVote::unsigned(peer, LockstepTick::from_legacy(req.tick), TickHash::from_bytes(hash_arr), role);
    let mut s = shared.lock().await;
    match &mut s.witness {
        None       => bad!("witness executor not configured; call POST /witness/setup first"),
        Some(exec) => match exec.submit_vote(vote) {
            Ok(()) => Json(json!({ "accepted": true, "peer_id": req.peer_id, "tick": req.tick })).into_response(),
            Err(e) => bad!(format!("{e}")),
        },
    }
}

pub async fn get_consensus(
    State(shared): State<SharedState>,
    Path(tick_num): Path<u64>,
) -> impl IntoResponse {
    let tick = LockstepTick::from_legacy(tick_num);
    let s = shared.lock().await;
    match &s.witness {
        None => bad!("witness executor not configured; call POST /witness/setup first"),
        Some(exec) => {
            let result = exec.evaluate_consensus(tick);
            let (result_str, hash, details) = match &result {
                ConsensusResult::Accepted { tick_hash, .. } => (
                    "accepted",
                    Some(hex::encode(tick_hash.as_bytes())),
                    json!({ "tick_hash": hex::encode(tick_hash.as_bytes()) }),
                ),
                ConsensusResult::Contested { authority_hash, mismatched_peers, replay_from_tick, .. } => (
                    "contested",
                    Some(hex::encode(authority_hash.as_bytes())),
                    json!({
                        "authority_hash":   hex::encode(authority_hash.as_bytes()),
                        "mismatched_peers": mismatched_peers.iter().map(|p| hex::encode(p.as_bytes())).collect::<Vec<_>>(),
                        "replay_from_tick": replay_from_tick.local_seq(),
                    }),
                ),
                ConsensusResult::Pending { votes_received, votes_required, .. } => (
                    "pending",
                    None,
                    json!({ "votes_received": votes_received, "votes_required": votes_required }),
                ),
            };
            Json(ConsensusResp { tick: tick_num, result: result_str, hash, details }).into_response()
        }
    }
}

// ─── WAC — World Asset Compiler ───────────────────────────────────────────────

/// Validate and compile an [`AssetBlueprint`] to [`AssetIR`].
///
/// The compiled result is stored in the in-memory [`AssetCache`] for
/// subsequent retrieval by semantic hash.
///
/// # Errors
/// - 400 if the blueprint fails validation.
/// - 422 if the compiler cannot process the spec.
pub async fn wac_compile(
    State(shared): State<SharedState>,
    Json(bp): Json<AssetBlueprint>,
) -> impl IntoResponse {
    // Validate first (cheap).
    if let Err(e) = validate(&bp) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": e.to_string() }))).into_response();
    }

    // Compile (deterministic).
    match compile(&bp) {
        Ok(ir) => {
            let key_hex = hex::encode(semantic_hash_with_seed(&bp));
            let mut s = shared.lock().await;
            s.asset_cache.insert(&bp, ir.clone());
            Json(json!({
                "ok":          true,
                "blueprint_id": ir.blueprint_id.to_string(),
                "ir_version":   ir.ir_version,
                "semantic_hash": key_hex,
                "asset":         ir.asset,
            })).into_response()
        }
        Err(e) => (StatusCode::UNPROCESSABLE_ENTITY,
                   Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

/// Look up a previously compiled asset by its 64-char hex semantic hash.
pub async fn wac_cache_get(
    State(shared): State<SharedState>,
    Path(hash_hex): Path<String>,
) -> impl IntoResponse {
    let bytes = match hex::decode(&hash_hex) {
        Ok(b) => b,
        Err(_) => return (StatusCode::BAD_REQUEST,
                          Json(json!({ "error": "invalid hex hash" }))).into_response(),
    };
    let key: [u8; 32] = match bytes.try_into() {
        Ok(k) => k,
        Err(_) => return (StatusCode::BAD_REQUEST,
                          Json(json!({ "error": "hash must be 32 bytes (64 hex chars)" }))).into_response(),
    };
    let s = shared.lock().await;
    match s.asset_cache.get_by_key(&key) {
        Some(ir) => Json(ir).into_response(),
        None     => (StatusCode::NOT_FOUND, Json(json!({ "error": "not in cache" }))).into_response(),
    }
}

/// Return WAC cache statistics.
pub async fn wac_cache_stats(State(shared): State<SharedState>) -> Json<serde_json::Value> {
    let s = shared.lock().await;
    Json(json!({
        "entries": s.asset_cache.len(),
        "capacity": bifrost_wac::cache::CACHE_CAPACITY,
    }))
}

// ─── World Director ────────────────────────────────────────────────────────────

/// Run one World Director tick given a [`PressureGraph`].
///
/// Returns the list of [`DirectorDecision`]s (blueprint + reason + tick).
/// Each blueprint can immediately be forwarded to `POST /wac/compile`.
pub async fn director_tick(
    State(shared): State<SharedState>,
    Json(pressure): Json<PressureGraph>,
) -> Json<serde_json::Value> {
    let mut s = shared.lock().await;
    let decisions = s.director.tick(&pressure);
    Json(json!({
        "decisions": decisions,
        "total_emitted": s.director.state.total_blueprints_emitted,
    }))
}

/// Return the World Director's recent decision history.
pub async fn director_history(State(shared): State<SharedState>) -> Json<serde_json::Value> {
    let s = shared.lock().await;
    let recent = s.director.recent_decisions(50);
    Json(json!({
        "decisions": recent,
        "total_emitted": s.director.state.total_blueprints_emitted,
        "config": s.director.config,
    }))
}

// ─── Nexus Voxel Kernel ────────────────────────────────────────────────────────

use nexus_voxel_kernel::bridge::{WacError, WacResult};
use nexus_voxel_kernel::core::ChunkPos;

/// Apply a WAC JSON document to the nexus voxel kernel.
///
/// Generates a `VoxelChunk` deterministically and caches it in the
/// `WorldRuntime`. Returns chunk metadata including position, biome,
/// fill count, and BLAKE3 state hash.
///
/// # Example
///
/// ```bash
/// curl -X POST http://localhost:8080/nexus/wac \
///   -H 'Content-Type: application/json' \
///   -d '{"type":"chunk","pos":{"x":0,"y":0,"z":0},"biome":"crimson_forest"}'
/// ```
pub async fn nexus_wac(
    State(shared): State<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let wac_str = match serde_json::to_string(&body) {
        Ok(s)  => s,
        Err(e) => return (StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("invalid JSON: {e}") }))).into_response(),
    };
    let mut s = shared.lock().await;
    match s.nexus_rt.apply(&wac_str) {
        Ok(WacResult::ChunkGenerated(chunk)) => {
            Json(json!({
                "ok":         true,
                "chunk_pos":  { "x": chunk.position.x, "y": chunk.position.y, "z": chunk.position.z },
                "biome":      chunk.meta.biome,
                "seed":       chunk.meta.seed,
                "fill_count": chunk.meta.fill_count,
                "state_hash": hex::encode(chunk.state_hash),
                "nav_passable": chunk.meta.nav_passable,
            })).into_response()
        }
        Ok(WacResult::RegionGenerated(chunks)) => {
            let summaries: Vec<_> = chunks.iter().map(|c| json!({
                "pos":        { "x": c.position.x, "y": c.position.y, "z": c.position.z },
                "biome":      c.meta.biome,
                "fill_count": c.meta.fill_count,
                "state_hash": hex::encode(&c.state_hash[..4]),
            })).collect();
            Json(json!({ "ok": true, "chunks": summaries, "count": summaries.len() })).into_response()
        }
        Ok(WacResult::BiomeRegistered(name)) => {
            Json(json!({ "ok": true, "registered_biome": name })).into_response()
        }
        Ok(WacResult::MaterialRegistered { name, id }) => {
            Json(json!({ "ok": true, "registered_material": name, "id": id })).into_response()
        }
        Err(WacError::Json(e)) => (StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("JSON error: {e}") }))).into_response(),
        Err(WacError::UnknownTerrain(t)) => (StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("unknown terrain style: {t}") }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

/// List all registered biomes (built-in + AI-registered).
pub async fn nexus_biomes(State(shared): State<SharedState>) -> Json<serde_json::Value> {
    let s = shared.lock().await;
    let names: Vec<&str> = s.nexus_rt.world.biomes.names().collect();
    Json(json!({ "biomes": names, "count": names.len() }))
}

/// Get chunk data from the nexus world cache.
pub async fn nexus_chunk(
    State(shared): State<SharedState>,
    Path((x, y, z)): Path<(i32, i32, i32)>,
) -> impl IntoResponse {
    let s = shared.lock().await;
    match s.nexus_rt.world.get_chunk(ChunkPos::new(x, y, z)) {
        None => (StatusCode::NOT_FOUND,
            Json(json!({ "error": "chunk not loaded — POST /nexus/wac first" }))).into_response(),
        Some(chunk) => Json(json!({
            "pos":        { "x": x, "y": y, "z": z },
            "biome":      chunk.meta.biome,
            "seed":       chunk.meta.seed,
            "fill_count": chunk.meta.fill_count,
            "surface_y":  chunk.meta.surface_y,
            "nav_passable": chunk.meta.nav_passable,
            "state_hash": hex::encode(chunk.state_hash),
        })).into_response(),
    }
}

/// Return nexus world statistics.
pub async fn nexus_world_stats(State(shared): State<SharedState>) -> Json<serde_json::Value> {
    let s = shared.lock().await;
    let w = &s.nexus_rt.world;
    let biome_names: Vec<&str> = w.biomes.names().collect();
    Json(json!({
        "chunks_loaded":  w.chunk_count(),
        "total_voxels":   w.total_voxels(),
        "biomes_available": biome_names.len(),
        "palette_size":   w.palette.len(),
        "pending_requests": s.nexus_rt.streamer.pending_count(),
        "generated_count":  s.nexus_rt.streamer.generated_count(),
    }))
}

/// Generate a demo chunk using a random biome to show nexus pipeline.
pub async fn nexus_demo(State(shared): State<SharedState>) -> impl IntoResponse {
    let demo_wacs = [
        r#"{"type":"chunk","pos":{"x":0,"y":0,"z":0},"biome":"crimson_forest"}"#,
        r#"{"type":"chunk","pos":{"x":0,"y":0,"z":1},"biome":"volcanic_wastes"}"#,
        r#"{"type":"chunk","pos":{"x":1,"y":0,"z":0},"biome":"crystal_caves"}"#,
    ];
    let mut results = Vec::new();
    let mut s = shared.lock().await;
    for wac in demo_wacs {
        match s.nexus_rt.apply(wac) {
            Ok(WacResult::ChunkGenerated(c)) => {
                results.push(json!({
                    "pos":   { "x": c.position.x, "y": c.position.y, "z": c.position.z },
                    "biome": c.meta.biome,
                    "fill":  c.meta.fill_count,
                    "hash":  hex::encode(&c.state_hash[..4]),
                }));
            }
            _ => {}
        }
    }
    Json(json!({
        "ok":     true,
        "chunks": results,
        "msg":    "Nexus voxel kernel: LLM → WAC → VoxelChunk pipeline working.",
    })).into_response()
}
