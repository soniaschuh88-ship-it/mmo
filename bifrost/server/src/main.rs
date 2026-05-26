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
        // Shared state + CORS (allow all for development)
        .with_state(shared)
        .layer(CorsLayer::permissive());

    tracing::info!("bifrost-server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
