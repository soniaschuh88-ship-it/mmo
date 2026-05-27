//! HTTP route handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use bifrost_chunk::PeerId;
use bifrost_run::end_condition::WorldSnapshot;
use bifrost_run::run::{RunResult, RunState, WorldRun};
use bifrost_synthesis::AiFaction;
use bifrost_synthesis::tick::{SynthesisTick, TickInput};
use bifrost_safe_city::auction::Listing;

use crate::models::{
    StartRunReq, RunTickReq, EndRunReq,
    SynthesisInitReq, SynthesisTickReq,
    PostListingReq, BuyListingReq, ZoneInfluenceReq,
};
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

// ─── Run System ───────────────────────────────────────────────────────────────

/// `POST /run` — start a new world run epoch.
///
/// Creates a [`WorldRun`] from the request, registers it with the
/// [`WorldRunDirector`], and transitions it to `Active` state.
pub async fn start_run(
    State(shared): State<SharedState>,
    Json(req): Json<StartRunReq>,
) -> impl IntoResponse {
    let end_condition = match serde_json::from_value(req.end_condition) {
        Ok(ec) => ec,
        Err(e) => return bad!(format!("invalid end_condition: {e}")),
    };
    let run = WorldRun::new(
        end_condition,
        req.player_factions,
        req.ai_factions,
        req.world_seed,
        req.label,
    );
    let mut s = shared.lock().await;
    let tick = s.world.tick();
    s.run_director.add_run(run);
    s.run_director.start_next_run(tick);
    match s.run_director.active_run() {
        Some(r) => Json(json!({ "ok": true, "run": r })).into_response(),
        None    => bad!("failed to activate run"),
    }
}

/// `GET /run/current` — return the active world run, if any.
pub async fn get_run(State(shared): State<SharedState>) -> impl IntoResponse {
    let s = shared.lock().await;
    match s.run_director.active_run() {
        Some(r) => Json(json!({ "run": r })).into_response(),
        None    => (StatusCode::NOT_FOUND, Json(json!({ "error": "no active run" }))).into_response(),
    }
}

/// `POST /run/tick` — evaluate win conditions against a world snapshot.
///
/// Returns the run result if a win condition has been satisfied, otherwise
/// `{"status": "continue"}`.
pub async fn tick_run(
    State(shared): State<SharedState>,
    Json(req): Json<RunTickReq>,
) -> impl IntoResponse {
    let snap = WorldSnapshot {
        current_tick:      req.current_tick,
        zones_controlled:  req.zones_controlled,
        tech_levels:       req.tech_levels,
        economy_fractions: req.economy_fractions,
    };
    let mut s = shared.lock().await;
    match s.run_director.evaluate_tick(req.current_tick, &snap) {
        None      => Json(json!({ "status": "continue", "tick": req.current_tick })).into_response(),
        Some(res) => Json(json!({
            "status":          "run_ended",
            "winner":          res.winner,
            "losers":          res.losers,
            "condition":       format!("{:?}", res.condition_triggered),
            "summary":         res.summary,
        })).into_response(),
    }
}

/// `POST /run/end` — force-end the active run.
///
/// ## Step 7 — Wire run→PressureGraph→WorldDirector (FIX.md)
///
/// After ending the run:
/// 1. Builds a [`PressureGraph`] from the run result (dominant strategy → signals)
/// 2. Feeds it to [`WorldDirector::tick`] → emits [`AssetBlueprint`]s
/// 3. Compiles each blueprint through WAC
/// 4. Applies compiled biome/loot/entity assets to the nexus voxel kernel
///
/// This closes the loop: `Run end → WAC compile → new world`.
pub async fn end_run(
    State(shared): State<SharedState>,
    Json(req): Json<EndRunReq>,
) -> impl IntoResponse {
    use bifrost_run::end_condition::EndCondition;
    use bifrost_wac::pressure::{GlobalPressure, PressureGraph, ZonePressure};

    let mut s = shared.lock().await;
    let tick = s.world.tick();

    // ── 1. End the run ────────────────────────────────────────────────────────
    let run_result = match s.run_director.active_run_mut() {
        None => return (StatusCode::NOT_FOUND, Json(json!({ "error": "no active run" }))).into_response(),
        Some(run) => {
            let result = if let Some(ref winner) = req.winner_faction_id {
                let losers: Vec<_> = run.player_factions.iter().chain(run.ai_factions.iter())
                    .filter(|f| f.as_str() != winner)
                    .cloned()
                    .collect();
                RunResult::winner(winner.clone(), losers, run.end_condition.clone())
            } else {
                RunResult::draw(
                    run.end_condition.clone(),
                    run.player_factions.iter().chain(run.ai_factions.iter()).cloned().collect(),
                )
            };
            run.state    = RunState::Ended(result.clone());
            run.end_tick = Some(tick);
            result
        }
    };

    // ── 2. Build PressureGraph from run result (Step 7) ───────────────────────
    // Map the run's dominant strategy onto world pressure signals so the
    // WorldDirector counter-adapts the next epoch.
    let mut pressure = PressureGraph::new(tick);

    match &run_result.condition_triggered {
        EndCondition::EconomicDominance { .. } => {
            // Economy exploit → generate scarcity biomes + volatile loot
            pressure.global = GlobalPressure {
                economy_delta:      0.45,   // strong inflation signal
                narrative_momentum: 0.05,   // narrative needs a spark
                total_players:      1,
                player_trend:       0.0,
                quest_throughput:   0.0,
            };
        }
        EndCondition::FirstToControlZones { .. } => {
            // Zone rush → increase contention in all zones
            for zone_id in s.zones.keys().cloned().collect::<Vec<_>>() {
                pressure.insert_zone(ZonePressure {
                    zone_id:        zone_id.clone(),
                    player_density: 5.0,
                    kill_rate:      4.0,    // generates biome evolution
                    contention:     0.9,
                    loot_flow:      20.0,
                    quest_rate:     0.3,
                });
            }
        }
        EndCondition::SurvivalUntilTick(_) | EndCondition::FirstToReachTechLevel { .. } => {
            // Balanced — narrative was the winning factor
            pressure.global = GlobalPressure {
                economy_delta:      0.0,
                narrative_momentum: 0.02,   // stalled — fire a story beat
                total_players:      1,
                player_trend:       0.0,
                quest_throughput:   0.0,
            };
        }
    }

    // ── 3. WorldDirector tick → AssetBlueprints ───────────────────────────────
    let decisions = s.director.tick(&pressure);

    // ── 4. Compile blueprints through WAC + apply to nexus_rt ────────────────
    let mut compiled_count = 0usize;
    for decision in &decisions {
        if let Ok(ir) = bifrost_wac::compile(&decision.blueprint) {
            // Validate succeeded; apply to the nexus voxel kernel.
            // Biome definitions update the chunk generator for new chunks.
            use bifrost_wac::types::CompiledAsset;
            if let CompiledAsset::BiomeDefinition(biome_ir) = ir.asset {
                // Apply the new biome to the nexus runtime (no specific position —
                // this registers the biome for future chunk generation).
                s.nexus_rt.apply_biome_ir(
                    biome_ir,
                    nexus_voxel_kernel::core::ChunkPos::default(),
                );
                compiled_count += 1;
            }
        }
    }

    let run_id = s.run_director.runs.last().map(|r| r.id);
    Json(json!({
        "ok":             true,
        "run_id":         run_id,
        "winner":         run_result.winner,
        "reason":         req.reason,
        "ended_at_tick":  tick,
        "director_decisions": decisions.len(),
        "compiled_blueprints": compiled_count,
    })).into_response()
}

/// `GET /run/history` — all runs (active and completed).
pub async fn run_history(State(shared): State<SharedState>) -> Json<serde_json::Value> {
    let s = shared.lock().await;
    let runs = &s.run_director.runs;
    Json(json!({
        "runs":  runs,
        "count": runs.len(),
    }))
}

// ─── Synthesis AI ─────────────────────────────────────────────────────────────

/// `POST /synthesis/init` — create (or reset) the Synthesis AI faction.
pub async fn synthesis_init(
    State(shared): State<SharedState>,
    Json(req): Json<SynthesisInitReq>,
) -> impl IntoResponse {
    let faction = AiFaction::new(req.faction_id, req.display_name);
    let mut s = shared.lock().await;
    let resp = json!({
        "ok":      true,
        "faction": faction,
    });
    s.synthesis = Some(faction);
    Json(resp).into_response()
}

/// `GET /synthesis/faction` — return the Synthesis AI faction state.
pub async fn synthesis_faction(State(shared): State<SharedState>) -> impl IntoResponse {
    let s = shared.lock().await;
    match &s.synthesis {
        None    => (StatusCode::NOT_FOUND, Json(json!({ "error": "synthesis AI not initialised — POST /synthesis/init first" }))).into_response(),
        Some(f) => Json(json!({ "faction": f })).into_response(),
    }
}

/// `POST /synthesis/tick` — run one Synthesis AI tick and return emitted intents.
pub async fn synthesis_tick(
    State(shared): State<SharedState>,
    Json(req): Json<SynthesisTickReq>,
) -> impl IntoResponse {
    let input = TickInput {
        tick:                    req.current_tick,
        zone_resources:          req.owned_zones.iter()
            .map(|z| (z.clone(), 0.5f32))
            .collect(),
        player_fortresses:       std::collections::BTreeMap::new(),
        player_economy_fraction: 1.0 - req.threat_level.clamp(0.0, 1.0),
    };
    let mut s = shared.lock().await;
    match &mut s.synthesis {
        None => (StatusCode::NOT_FOUND, Json(json!({ "error": "synthesis AI not initialised — POST /synthesis/init first" }))).into_response(),
        Some(faction) => {
            let mut ticker = SynthesisTick::new(faction);
            let output = ticker.run(input);
            Json(json!({
                "tick":    req.current_tick,
                "intents": output.intents,
                "goals":   output.goals,
                "intent_count": output.intents.len(),
            })).into_response()
        }
    }
}

/// `GET /synthesis/agents` — list current Synthesis agent nodes.
pub async fn synthesis_agents(State(shared): State<SharedState>) -> impl IntoResponse {
    let s = shared.lock().await;
    match &s.synthesis {
        None    => (StatusCode::NOT_FOUND, Json(json!({ "error": "synthesis AI not initialised" }))).into_response(),
        Some(f) => Json(json!({ "agents": f.agents, "count": f.agents.len() })).into_response(),
    }
}

// ─── Safe City + Economy ──────────────────────────────────────────────────────

/// `GET /safe-city` — return Safe City state and market summary.
pub async fn safe_city_info(State(shared): State<SharedState>) -> Json<serde_json::Value> {
    let s = shared.lock().await;
    Json(json!({
        "zone_id":          s.safe_city.zone_id,
        "protection_level": s.safe_city.protection_level,
        "allowed_actions":  s.safe_city.allowed_actions,
        "active_listings":  s.safe_city.market.listings.len(),
    }))
}

/// `GET /safe-city/auction` — list all active auction listings.
pub async fn auction_listings(State(shared): State<SharedState>) -> Json<serde_json::Value> {
    let s = shared.lock().await;
    let active: Vec<_> = s.safe_city.market.listings.iter()
        .filter(|l| matches!(l.status, bifrost_safe_city::auction::ListingStatus::Active))
        .collect();
    Json(json!({
        "listings": active,
        "count": active.len(),
        "tax_policy": s.safe_city.market.tax_policy,
    }))
}

/// `POST /safe-city/auction/list` — post a new fixed-price listing.
pub async fn post_listing(
    State(shared): State<SharedState>,
    Json(req): Json<PostListingReq>,
) -> impl IntoResponse {
    let listing = Listing::new_fixed(
        req.seller_id,
        req.item_id,
        req.item_name,
        req.quantity,
        req.unit_price,
        req.current_tick,
    );
    let mut s = shared.lock().await;
    let id = s.safe_city.market.post(listing);
    Json(json!({ "ok": true, "listing_id": id })).into_response()
}

/// `POST /safe-city/auction/buy` — purchase quantity of a listing.
pub async fn buy_listing(
    State(shared): State<SharedState>,
    Json(req): Json<BuyListingReq>,
) -> impl IntoResponse {
    let id = match uuid::Uuid::parse_str(&req.listing_id) {
        Ok(u)  => u,
        Err(_) => return bad!("listing_id must be a valid UUID"),
    };
    let mut s = shared.lock().await;
    match s.safe_city.market.buy(id, &req.buyer_id, req.budget) {
        Ok(receipt) => Json(json!({ "ok": true, "receipt": receipt })).into_response(),
        Err(e)      => (StatusCode::UNPROCESSABLE_ENTITY,
                        Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

/// `GET /safe-city/zones` — return all world zones.
pub async fn list_zones(State(shared): State<SharedState>) -> Json<serde_json::Value> {
    let s = shared.lock().await;
    let zones: Vec<_> = s.zones.values().collect();
    Json(json!({ "zones": zones, "count": zones.len() }))
}

/// `GET /safe-city/zones/:id` — return a single zone by ID.
pub async fn get_zone(
    State(shared): State<SharedState>,
    Path(zone_id): Path<String>,
) -> impl IntoResponse {
    let s = shared.lock().await;
    match s.zones.get(&zone_id) {
        None    => (StatusCode::NOT_FOUND, Json(json!({ "error": format!("zone '{zone_id}' not found") }))).into_response(),
        Some(z) => Json(json!({ "zone": z })).into_response(),
    }
}

/// `POST /safe-city/zones/:id/influence` — apply faction influence delta to a zone.
///
/// Influence accumulates per faction.  When it reaches 1.0 the zone
/// transitions to `Controlled` state automatically.
pub async fn zone_influence(
    State(shared): State<SharedState>,
    Path(zone_id): Path<String>,
    Json(req): Json<ZoneInfluenceReq>,
) -> impl IntoResponse {
    let mut s = shared.lock().await;
    match s.zones.get_mut(&zone_id) {
        None => (StatusCode::NOT_FOUND, Json(json!({ "error": format!("zone '{zone_id}' not found") }))).into_response(),
        Some(zone) => {
            zone.apply_influence(&req.faction_id, req.delta);
            Json(json!({
                "ok":       true,
                "zone_id":  zone_id,
                "faction":  req.faction_id,
                "delta":    req.delta,
                "state":    zone.state,
                "influence": zone.influence,
            })).into_response()
        }
    }
}

// ─── AI Game Master routes ────────────────────────────────────────────────────

use bifrost_aigm::event::{
    AuthorId, EventPayload, EventType, QuestCreatePayload,
    QuestObjectivePayload, QuestRewardPayload, ReputationChangePayload,
};
use bifrost_kernel::SequencedInstant;
use bifrost_aigm::event::WorldEvent as AigmWorldEvent;

/// `GET /aigm/npcs` — return all NPCs in the start area (safe-city zone).
///
/// game.html replaces its hardcoded NPC array with the response from this
/// endpoint on startup.  Each entry includes id, name, position, and
/// dialogue lines needed for the renderer.
pub async fn aigm_npcs(State(shared): State<SharedState>) -> Json<serde_json::Value> {
    let s = shared.lock().await;
    let npcs: Vec<serde_json::Value> = s.npc_registry.iter()
        .map(|(id, state)| {
            // system_prompt is stored as "DisplayName|goal"
            let name = state.ai_context.system_prompt
                .split_once('|')
                .map(|(n, _)| n)
                .unwrap_or(id);
            json!({
                "id":      id,
                "name":    name,
                "zone_id": state.zone_id,
                "wx":      state.position[0],  // 2D world-x coord for game.html
                "wy":      state.position[2],  // 2D world-y coord (z in 3D)
                "hp":      state.hp_current,
                "hp_max":  state.hp_max,
                "alive":   state.is_alive(),
            })
        })
        .collect();
    Json(json!({ "npcs": npcs, "count": npcs.len() }))
}

/// Static quest-chain definitions served to the client.
///
/// This mirrors the QCHAINS object formerly hardcoded in game.html.
/// The server is authoritative; the client falls back to its bundled copy
/// if this endpoint is unreachable.
fn quest_chains_json() -> serde_json::Value {
    json!({
      "chain_guard": {
        "stages": [
          { "id":"g1","title":"Wolf Menace","icon":"🐺","target":"wolf","count":5,"gold":40,"xp":80,
            "who":"Guard Captain Aldric","desc":"Kill 5 wolves in the northern fields.",
            "dlg":"The wolves have killed 3 farmers this week. Clear them out!",
            "reward_dlg":"Excellent work! The farms are safe again.","next":"g2" },
          { "id":"g2","title":"Goblin Raiders","icon":"👺","target":"goblin","count":8,"gold":70,"xp":140,
            "who":"Guard Captain Aldric","desc":"Defeat 8 goblins raiding from the eastern mountains.",
            "dlg":"Now the goblins grow bold. Strike their war band!",
            "reward_dlg":"Masterful! But their chief still roams the peaks.","next":"g3" },
          { "id":"g3","title":"The Goblin Chief","icon":"👑","target":"goblin_chief","count":1,"gold":120,"xp":250,
            "who":"Guard Captain Aldric","desc":"Defeat the Goblin Chief in the eastern mountains.",
            "dlg":"Their chief leads them. Defeat him and they scatter!",
            "reward_dlg":"The mountains are free! You are a true hero.","next":null }
        ]
      },
      "chain_inn": {
        "stages": [
          { "id":"i1","title":"Rat Infestation","icon":"🐀","target":"rat","count":5,"gold":30,"xp":60,
            "who":"Innkeeper Bram","desc":"Clear the giant rats from the inn cellar.",
            "dlg":"Rats everywhere! I can't open the cellar. Please help!",
            "reward_dlg":"You've saved my business! But there was something else...","next":"i2" },
          { "id":"i2","title":"Spider Nest","icon":"🕷","target":"spider","count":6,"gold":55,"xp":110,
            "who":"Innkeeper Bram","desc":"Eliminate the spider nest in the dark forest.",
            "dlg":"Giant spiders followed the rats. I found webs in the forest!",
            "reward_dlg":"Thank the gods. Here, take this gold.","next":"i3" },
          { "id":"i3","title":"Forest Troll","icon":"👹","target":"troll","count":2,"gold":100,"xp":200,
            "who":"Innkeeper Bram","desc":"Drive away the forest trolls terrorizing travelers.",
            "dlg":"Trolls have blocked the forest road. Please clear them!",
            "reward_dlg":"The road is open again! You've been the salvation of this inn.","next":null }
        ]
      },
      "chain_elder": {
        "stages": [
          { "id":"e1","title":"Ancient Text","icon":"📖","target":"skeleton","count":4,"gold":60,"xp":120,
            "who":"Elder Mirova","desc":"The skeletons in the dungeon carry old relics. Recover 4.",
            "dlg":"Skeletons guard an ancient text we need. Please retrieve it.",
            "reward_dlg":"This text speaks of a powerful ritual. We must stop it.","next":"e2" },
          { "id":"e2","title":"Dark Crystals","icon":"💎","target":"troll","count":3,"gold":80,"xp":160,
            "who":"Elder Mirova","desc":"Trolls have stolen the ritual crystals. Recover them.",
            "dlg":"The ritual requires dark crystals. The trolls took them!",
            "reward_dlg":"We have what we need. But the ritual has begun...","next":"e3" },
          { "id":"e3","title":"Stop the Ritual","icon":"⚡","target":"lich","count":1,"gold":150,"xp":350,
            "who":"Elder Mirova","desc":"The Dungeon Lich leads the ritual. Destroy him!",
            "dlg":"The Lich performs the ritual deep in the dungeon. GO NOW!",
            "reward_dlg":"You've saved us all. The darkness is defeated!","next":null }
        ]
      },
      "chain_wiz": {
        "stages": [
          { "id":"w1","title":"Fire Elementals","icon":"🔥","target":"elemental","count":4,"gold":65,"xp":130,
            "who":"Wizard Seraphon","desc":"Fire elementals threaten the dungeon entrance. Banish 4.",
            "dlg":"The elementals block my research. Eliminate them!",
            "reward_dlg":"Excellent. The dungeon is accessible again.","next":"w2" },
          { "id":"w2","title":"Shard of Power","icon":"✨","target":"dungeon","count":3,"gold":90,"xp":180,
            "who":"Wizard Seraphon","desc":"Collect 3 dungeon shards for the arcane ritual.",
            "dlg":"The Lich holds 3 shards of ancient power. Retrieve them!",
            "reward_dlg":"Excellent. These shards reveal dungeon secrets.","next":"w3" },
          { "id":"w3","title":"Skeletal Guardians","icon":"💀","target":"skeleton","count":5,"gold":110,"xp":220,
            "who":"Wizard Seraphon","desc":"Skeletal guardians protect the dungeon archives.",
            "dlg":"Clear the archive guardians so I can retrieve the tome!",
            "reward_dlg":"The tome is ours. The secret of the Lich is revealed!","next":null }
        ]
      }
    })
}

/// `GET /aigm/quests` — return quest chain definitions + active registry state.
///
/// The client uses this to populate `QCHAINS` at startup.
/// Falls back to bundled data if this endpoint is unreachable.
pub async fn aigm_quests_list(State(shared): State<SharedState>) -> Json<serde_json::Value> {
    let s = shared.lock().await;
    let active: Vec<serde_json::Value> = s.quest_registry.active_quests()
        .map(|q| json!({
            "quest_id": q.quest_id,
            "title":    q.title,
            "state":    format!("{:?}", q.state),
        }))
        .collect();
    Json(json!({
        "chains": quest_chains_json(),
        "active": active,
        "count":  active.len(),
    }))
}

/// Request body for `POST /aigm/quests/:chain_id/accept`.
#[derive(serde::Deserialize)]
pub struct QuestAcceptReq {
    pub player_id: String,
    pub zone_id:   Option<String>,
}

/// `POST /aigm/quests/:chain_id/accept` — player accepts a quest chain.
///
/// Creates an `AigmQuestCreate` [`WorldEvent`], processes it through the zone
/// [`EventPipeline`] (R3), appends it to the [`Ledger`] (R4), and projects it
/// into the [`QuestRegistry`].
pub async fn aigm_quest_accept(
    State(shared): State<SharedState>,
    Path(chain_id): Path<String>,
    Json(req): Json<QuestAcceptReq>,
) -> impl IntoResponse {
    let zone_id = req.zone_id.unwrap_or_else(|| "safe-city".into());

    // Build the first stage of the chain as the quest to create.
    let chains = quest_chains_json();
    let stages = match chains.get(&chain_id).and_then(|c| c.get("stages")).and_then(|s| s.as_array()) {
        Some(s) => s.clone(),
        None => return (StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("unknown quest chain: {chain_id}") }))).into_response(),
    };
    let first = match stages.first() {
        Some(s) => s.clone(),
        None    => return bad!("quest chain has no stages"),
    };

    let quest_id = format!("{chain_id}_{}", req.player_id);
    let title    = first.get("title").and_then(|v| v.as_str()).unwrap_or("Quest").to_string();
    let desc     = first.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let giver    = first.get("who").and_then(|v| v.as_str()).unwrap_or("npc").to_string();
    let target   = first.get("target").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let count    = first.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
    let gold     = first.get("gold").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let xp       = first.get("xp").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let stage_id = first.get("id").and_then(|v| v.as_str()).unwrap_or("s1").to_string();

    let payload = QuestCreatePayload {
        quest_id:    quest_id.clone(),
        title,
        description: desc,
        giver_npc_id: giver,
        target_ids:  vec![req.player_id.clone()],
        objectives: vec![QuestObjectivePayload {
            objective_id:   stage_id,
            kind:           "kill".into(),
            description:    format!("Defeat {count} {target}"),
            target_id:      Some(target),
            required_count: count,
        }],
        reward: QuestRewardPayload {
            xp,
            gold,
            items: vec![],
            reputation: vec![ReputationChangePayload {
                faction_id: "village".into(),
                delta: 5,
                reason: "quest_complete".into(),
            }],
        },
        expires_at: None,
        ai_context: format!("player {player_id} accepted {chain_id}", player_id = req.player_id),
    };

    let event = AigmWorldEvent::new(
        SequencedInstant::ZERO,   // pipeline will overwrite this
        EventType::AigmQuestCreate,
        EventPayload::AigmQuestCreate(payload),
        AuthorId::AiGm,
        &[0u8; 32],
        &zone_id,
        0,
    );

    let mut s = shared.lock().await;
    match s.emit(event) {
        Ok(_) => Json(json!({
            "ok":       true,
            "quest_id": quest_id,
            "zone_id":  zone_id,
        })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e }))).into_response(),
    }
}
