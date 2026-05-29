//! # bifrost-server
//!
//! HTTP REST server exposing the Bifrost Layer.
//!
//! ## Run
//!
//! ```bash
//! cargo run -p bifrost-server -- --port 8080
//! # or
//! PORT=8080 cargo run -p bifrost-server
//! ```
//!
//! ## Quick start
//!
//! ```bash
//! # Full pipeline demo (no setup needed)
//! curl -X POST http://localhost:8080/demo
//!
//! # Register peers
//! curl -X POST http://localhost:8080/peers \
//!      -H 'Content-Type: application/json' \
//!      -d '{"peer_id":"0101010101010101010101010101010101010101010101010101010101010101"}'
//! ```

use std::net::SocketAddr;
use axum::{
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
mod admin;
mod api;
mod models;
mod state;

#[tokio::main]
async fn main() {
    // Tracing setup (RUST_LOG=bifrost_server=debug,tower_http=debug)
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let shared = state::new_shared();

    let app = Router::new()
        // Meta
        .route("/",       get(api::root))
        .route("/health", get(api::health))
        .route("/state",  get(api::get_state))
        // Demo
        .route("/demo", post(api::demo))
        // Peers
        .route("/peers",          post(api::register_peer))
        .route("/peers/:peer_id", delete(api::evict_peer))
        // Tick
        .route("/tick",         get(api::get_tick))
        .route("/tick/input",   post(api::submit_input))
        .route("/tick/ack",     post(api::ack_tick))
        .route("/tick/advance", post(api::advance_tick))
        // World
        .route("/world/state",       get(api::world_state))
        .route("/world/instruction", post(api::execute_instruction))
        // Witness
        .route("/witness/setup",           post(api::setup_witness))
        .route("/witness/vote",            post(api::submit_witness_vote))
        .route("/witness/consensus/:tick", get(api::get_consensus))
        // WAC — World Asset Compiler
        .route("/wac/compile",             post(api::wac_compile))
        .route("/wac/cache/:hash",         get(api::wac_cache_get))
        .route("/wac/cache",               get(api::wac_cache_stats))
        // World Director
        .route("/wac/director/tick",       post(api::director_tick))
        .route("/wac/director/history",    get(api::director_history))
        // Nexus Voxel Kernel
        .route("/nexus/wac",           post(api::nexus_wac))
        .route("/nexus/biomes",        get(api::nexus_biomes))
        .route("/nexus/chunk/:x/:y/:z",get(api::nexus_chunk))
        .route("/nexus/world",         get(api::nexus_world_stats))
        .route("/nexus/demo",          post(api::nexus_demo))
        // Run System — discrete world epoch lifecycle
        .route("/run",                 post(api::start_run))
        .route("/run/current",         get(api::get_run))
        .route("/run/tick",            post(api::tick_run))
        .route("/run/end",             post(api::end_run))
        .route("/run/history",         get(api::run_history))
        // Synthesis AI — Synthesis faction tick + state
        .route("/synthesis/init",      post(api::synthesis_init))
        .route("/synthesis/faction",   get(api::synthesis_faction))
        .route("/synthesis/tick",      post(api::synthesis_tick))
        .route("/synthesis/agents",    get(api::synthesis_agents))
        // AI Game Master — quests + NPCs (Steps 5+6 / R3)
        .route("/aigm/quests",                        get(api::aigm_quests_list))
        .route("/aigm/quests/:chain_id/accept",       post(api::aigm_quest_accept))
        .route("/aigm/npcs",                          get(api::aigm_npcs))
        // Admin API — world-data.json read/write for the admin panel
        .route("/admin-api/world",                              get(admin::get_world).put(admin::put_world))
        .route("/admin-api/biomes",                             get(admin::get_biomes).post(admin::create_biome))
        .route("/admin-api/biomes/:id",                         axum::routing::put(admin::update_biome).delete(admin::delete_biome))
        .route("/admin-api/story",                              get(admin::get_story))
        .route("/admin-api/story/arcs",                         post(admin::create_arc))
        .route("/admin-api/story/arcs/:id",                     axum::routing::put(admin::update_arc).delete(admin::delete_arc))
        .route("/admin-api/story/arcs/:arc_id/beats",           post(admin::create_beat))
        .route("/admin-api/story/arcs/:arc_id/beats/:beat_id",  axum::routing::put(admin::update_beat).delete(admin::delete_beat))
        .route("/admin-api/npcs",                               get(admin::get_npcs).post(admin::create_npc))
        .route("/admin-api/npcs/:id",                           axum::routing::put(admin::update_npc).delete(admin::delete_npc))
        .route("/admin-api/quests",                             get(admin::get_quests).post(admin::create_quest))
        .route("/admin-api/quests/:id",                         axum::routing::put(admin::update_quest).delete(admin::delete_quest))
        .route("/admin-api/loot/monsters",                      get(admin::get_monsters).post(admin::create_monster))
        .route("/admin-api/loot/monsters/:id",                  axum::routing::put(admin::update_monster).delete(admin::delete_monster))
        .route("/admin-api/loot/items",                         get(admin::get_loot_items).post(admin::create_loot_item))
        .route("/admin-api/loot/items/:id",                     axum::routing::put(admin::update_loot_item).delete(admin::delete_loot_item))
        // Safe City — economy, auction house, zone control
        .route("/safe-city",                          get(api::safe_city_info))
        .route("/safe-city/auction",                  get(api::auction_listings))
        .route("/safe-city/auction/list",             post(api::post_listing))
        .route("/safe-city/auction/buy",              post(api::buy_listing))
        .route("/safe-city/zones",                    get(api::list_zones))
        .route("/safe-city/zones/:id",                get(api::get_zone))
        .route("/safe-city/zones/:id/influence",      post(api::zone_influence))
        // Shared state + CORS (allow all for development)
        .with_state(shared)
        .layer(CorsLayer::permissive());

    tracing::info!("bifrost-server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
