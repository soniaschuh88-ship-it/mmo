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
    let tick      = LockstepTick(0);
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
    steps.push(format!("tick advanced: {} → new_tick={}", tick_advanced, sched.current_tick().0));

    Json(DemoResult {
        peers: 3,
        instructions: instr_count,
        voxels_before,
        voxels_after,
        state_hash:    hex::encode(result.state_hash),
        consensus:     consensus_str,
        tick_advanced,
        new_tick:      sched.current_tick().0,
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
        current_tick:  s.scheduler.current_tick().0,
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
    let tick = LockstepTick(req.tick);

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
    s.scheduler.record_ack(peer, LockstepTick(req.tick));
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
                current_tick:         current.0,
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
                current_tick:         s.scheduler.current_tick().0,
                completed_tick:       Some(adv.completed_tick.0),
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
    let vote = WitnessVote::unsigned(peer, LockstepTick(req.tick), TickHash::from_bytes(hash_arr), role);
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
    let tick = LockstepTick(tick_num);
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
                        "replay_from_tick": replay_from_tick.0,
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
