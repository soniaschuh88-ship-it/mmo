//! Admin API handlers — reads and writes `app/world-data.json`.
//!
//! All endpoints operate on the live `world-data.json` file that the Yew
//! admin panel (`bifrost/admin`) uses.  JSON is read fresh on every GET and
//! written atomically on every mutation.
//!
//! Routes registered in `main.rs` under the `/admin-api/` prefix.

use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};
use tokio::fs;

/// Path to the world-data file relative to the CWD of bifrost-server.
/// When running from the workspace root: `cargo run -p bifrost-server`
/// the file is at `app/world-data.json`.
const WORLD_DATA_PATH: &str = "app/world-data.json";

// ─── File helpers ─────────────────────────────────────────────────────────────

async fn read_data() -> Result<Value, (StatusCode, Json<Value>)> {
    let raw = fs::read_to_string(WORLD_DATA_PATH).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("cannot read world-data.json: {e}") })),
        )
    })?;
    serde_json::from_str(&raw).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("cannot parse world-data.json: {e}") })),
        )
    })
}

async fn write_data(data: &Value) -> Result<(), (StatusCode, Json<Value>)> {
    let raw = serde_json::to_string_pretty(data).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("cannot serialise: {e}") })),
        )
    })?;
    fs::write(WORLD_DATA_PATH, raw).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("cannot write world-data.json: {e}") })),
        )
    })
}

// ─── World ────────────────────────────────────────────────────────────────────

/// `GET /admin-api/world`
pub async fn get_world() -> impl IntoResponse {
    match read_data().await {
        Err(e) => e.into_response(),
        Ok(d)  => Json(d["world"].clone()).into_response(),
    }
}

/// `PUT /admin-api/world`
pub async fn put_world(Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await {
        Err(e) => return e.into_response(),
        Ok(d)  => d,
    };
    data["world"] = body;
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    Json(data["world"].clone()).into_response()
}

// ─── Biomes ───────────────────────────────────────────────────────────────────

/// `GET /admin-api/biomes`
pub async fn get_biomes() -> impl IntoResponse {
    match read_data().await {
        Err(e) => e.into_response(),
        Ok(d)  => Json(d["biomes"].clone()).into_response(),
    }
}

/// `POST /admin-api/biomes`
pub async fn create_biome(Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if data["biomes"].is_null() { data["biomes"] = json!([]); }
    let arr = data["biomes"].as_array_mut().unwrap();
    // Check for duplicate id
    let id = body.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if arr.iter().any(|b| b["id"] == id) {
        return (StatusCode::CONFLICT, Json(json!({ "error": format!("biome '{id}' already exists") }))).into_response();
    }
    arr.push(body.clone());
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::CREATED, Json(body)).into_response()
}

/// `PUT /admin-api/biomes/:id`
pub async fn update_biome(Path(id): Path<String>, Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    let arr = match data["biomes"].as_array_mut() {
        Some(a) => a,
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": "no biomes found" }))).into_response(),
    };
    match arr.iter_mut().find(|b| b["id"] == id) {
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": format!("biome '{id}' not found") }))).into_response(),
        Some(b) => *b = body.clone(),
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    Json(body).into_response()
}

/// `DELETE /admin-api/biomes/:id`
pub async fn delete_biome(Path(id): Path<String>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if let Some(arr) = data["biomes"].as_array_mut() {
        arr.retain(|b| b["id"] != id);
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::NO_CONTENT, ()).into_response()
}

// ─── Story ────────────────────────────────────────────────────────────────────

/// `GET /admin-api/story`
pub async fn get_story() -> impl IntoResponse {
    match read_data().await {
        Err(e) => e.into_response(),
        Ok(d)  => Json(d["story"].clone()).into_response(),
    }
}

/// `POST /admin-api/story/arcs`
pub async fn create_arc(Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if data["story"]["arcs"].is_null() {
        data["story"] = json!({ "worldMood": "calm", "arcs": [] });
    }
    let arr = data["story"]["arcs"].as_array_mut().unwrap();
    let id = body.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if arr.iter().any(|a| a["id"] == id) {
        return (StatusCode::CONFLICT, Json(json!({ "error": format!("arc '{id}' already exists") }))).into_response();
    }
    arr.push(body.clone());
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::CREATED, Json(body)).into_response()
}

/// `PUT /admin-api/story/arcs/:id`
pub async fn update_arc(Path(id): Path<String>, Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    let arr = match data["story"]["arcs"].as_array_mut() {
        Some(a) => a,
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": "no arcs" }))).into_response(),
    };
    match arr.iter_mut().find(|a| a["id"] == id) {
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": format!("arc '{id}' not found") }))).into_response(),
        Some(a) => *a = body.clone(),
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    Json(body).into_response()
}

/// `DELETE /admin-api/story/arcs/:id`
pub async fn delete_arc(Path(id): Path<String>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if let Some(arr) = data["story"]["arcs"].as_array_mut() {
        arr.retain(|a| a["id"] != id);
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::NO_CONTENT, ()).into_response()
}

/// `POST /admin-api/story/arcs/:arc_id/beats`
pub async fn create_beat(Path(arc_id): Path<String>, Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    let arr = match data["story"]["arcs"].as_array_mut() {
        Some(a) => a,
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": "no arcs" }))).into_response(),
    };
    let arc = match arr.iter_mut().find(|a| a["id"] == arc_id) {
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": format!("arc '{arc_id}' not found") }))).into_response(),
        Some(a) => a,
    };
    if arc["beats"].is_null() { arc["beats"] = json!([]); }
    arc["beats"].as_array_mut().unwrap().push(body.clone());
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::CREATED, Json(body)).into_response()
}

/// `PUT /admin-api/story/arcs/:arc_id/beats/:beat_id`
pub async fn update_beat(Path((arc_id, beat_id)): Path<(String, String)>, Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    let arr = match data["story"]["arcs"].as_array_mut() {
        Some(a) => a,
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": "no arcs" }))).into_response(),
    };
    let arc = match arr.iter_mut().find(|a| a["id"] == arc_id) {
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": "arc not found" }))).into_response(),
        Some(a) => a,
    };
    let beats = match arc["beats"].as_array_mut() {
        Some(b) => b,
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": "no beats" }))).into_response(),
    };
    match beats.iter_mut().find(|b| b["id"] == beat_id) {
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": format!("beat '{beat_id}' not found") }))).into_response(),
        Some(b) => *b = body.clone(),
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    Json(body).into_response()
}

/// `DELETE /admin-api/story/arcs/:arc_id/beats/:beat_id`
pub async fn delete_beat(Path((arc_id, beat_id)): Path<(String, String)>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if let Some(arc) = data["story"]["arcs"].as_array_mut()
        .and_then(|arcs| arcs.iter_mut().find(|a| a["id"] == arc_id))
    {
        if let Some(beats) = arc["beats"].as_array_mut() {
            beats.retain(|b| b["id"] != beat_id);
        }
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::NO_CONTENT, ()).into_response()
}

// ─── NPCs ─────────────────────────────────────────────────────────────────────

/// `GET /admin-api/npcs`
pub async fn get_npcs() -> impl IntoResponse {
    match read_data().await {
        Err(e) => e.into_response(),
        Ok(d)  => Json(d["npcs"].clone()).into_response(),
    }
}

/// `POST /admin-api/npcs`
pub async fn create_npc(Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if data["npcs"].is_null() { data["npcs"] = json!([]); }
    let arr = data["npcs"].as_array_mut().unwrap();
    let id = body.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if arr.iter().any(|n| n["id"] == id) {
        return (StatusCode::CONFLICT, Json(json!({ "error": format!("npc '{id}' already exists") }))).into_response();
    }
    arr.push(body.clone());
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::CREATED, Json(body)).into_response()
}

/// `PUT /admin-api/npcs/:id`
pub async fn update_npc(Path(id): Path<String>, Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    let arr = match data["npcs"].as_array_mut() {
        Some(a) => a,
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": "no npcs" }))).into_response(),
    };
    match arr.iter_mut().find(|n| n["id"] == id) {
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": format!("npc '{id}' not found") }))).into_response(),
        Some(n) => *n = body.clone(),
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    Json(body).into_response()
}

/// `DELETE /admin-api/npcs/:id`
pub async fn delete_npc(Path(id): Path<String>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if let Some(arr) = data["npcs"].as_array_mut() {
        arr.retain(|n| n["id"] != id);
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::NO_CONTENT, ()).into_response()
}

// ─── Quests ───────────────────────────────────────────────────────────────────

/// `GET /admin-api/quests`
pub async fn get_quests() -> impl IntoResponse {
    match read_data().await {
        Err(e) => e.into_response(),
        Ok(d)  => Json(d["quests"].clone()).into_response(),
    }
}

/// `POST /admin-api/quests`
pub async fn create_quest(Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if data["quests"].is_null() { data["quests"] = json!([]); }
    let arr = data["quests"].as_array_mut().unwrap();
    let id = body.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if arr.iter().any(|q| q["id"] == id) {
        return (StatusCode::CONFLICT, Json(json!({ "error": format!("quest '{id}' already exists") }))).into_response();
    }
    arr.push(body.clone());
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::CREATED, Json(body)).into_response()
}

/// `PUT /admin-api/quests/:id`
pub async fn update_quest(Path(id): Path<String>, Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    let arr = match data["quests"].as_array_mut() {
        Some(a) => a,
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": "no quests" }))).into_response(),
    };
    match arr.iter_mut().find(|q| q["id"] == id) {
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": format!("quest '{id}' not found") }))).into_response(),
        Some(q) => *q = body.clone(),
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    Json(body).into_response()
}

/// `DELETE /admin-api/quests/:id`
pub async fn delete_quest(Path(id): Path<String>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if let Some(arr) = data["quests"].as_array_mut() {
        arr.retain(|q| q["id"] != id);
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::NO_CONTENT, ()).into_response()
}

// ─── Loot — Monsters ─────────────────────────────────────────────────────────

/// `GET /admin-api/loot/monsters`
pub async fn get_monsters() -> impl IntoResponse {
    match read_data().await {
        Err(e) => e.into_response(),
        Ok(d)  => Json(d["monsters"].clone()).into_response(),
    }
}

/// `POST /admin-api/loot/monsters`
pub async fn create_monster(Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if data["monsters"].is_null() { data["monsters"] = json!([]); }
    let arr = data["monsters"].as_array_mut().unwrap();
    let id = body.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if arr.iter().any(|m| m["id"] == id) {
        return (StatusCode::CONFLICT, Json(json!({ "error": format!("monster '{id}' already exists") }))).into_response();
    }
    arr.push(body.clone());
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::CREATED, Json(body)).into_response()
}

/// `PUT /admin-api/loot/monsters/:id`
pub async fn update_monster(Path(id): Path<String>, Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    let arr = match data["monsters"].as_array_mut() {
        Some(a) => a,
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": "no monsters" }))).into_response(),
    };
    match arr.iter_mut().find(|m| m["id"] == id) {
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": format!("monster '{id}' not found") }))).into_response(),
        Some(m) => *m = body.clone(),
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    Json(body).into_response()
}

/// `DELETE /admin-api/loot/monsters/:id`
pub async fn delete_monster(Path(id): Path<String>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if let Some(arr) = data["monsters"].as_array_mut() {
        arr.retain(|m| m["id"] != id);
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::NO_CONTENT, ()).into_response()
}

// ─── Loot — Items ─────────────────────────────────────────────────────────────

/// `GET /admin-api/loot/items`
pub async fn get_loot_items() -> impl IntoResponse {
    match read_data().await {
        Err(e) => e.into_response(),
        Ok(d)  => Json(d["lootItems"].clone()).into_response(),
    }
}

/// `POST /admin-api/loot/items`
pub async fn create_loot_item(Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if data["lootItems"].is_null() { data["lootItems"] = json!([]); }
    let arr = data["lootItems"].as_array_mut().unwrap();
    let id = body.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if arr.iter().any(|i| i["id"] == id) {
        return (StatusCode::CONFLICT, Json(json!({ "error": format!("item '{id}' already exists") }))).into_response();
    }
    arr.push(body.clone());
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::CREATED, Json(body)).into_response()
}

/// `PUT /admin-api/loot/items/:id`
pub async fn update_loot_item(Path(id): Path<String>, Json(body): Json<Value>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    let arr = match data["lootItems"].as_array_mut() {
        Some(a) => a,
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": "no items" }))).into_response(),
    };
    match arr.iter_mut().find(|i| i["id"] == id) {
        None    => return (StatusCode::NOT_FOUND, Json(json!({ "error": format!("item '{id}' not found") }))).into_response(),
        Some(i) => *i = body.clone(),
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    Json(body).into_response()
}

/// `DELETE /admin-api/loot/items/:id`
pub async fn delete_loot_item(Path(id): Path<String>) -> impl IntoResponse {
    let mut data = match read_data().await { Err(e) => return e.into_response(), Ok(d) => d };
    if let Some(arr) = data["lootItems"].as_array_mut() {
        arr.retain(|i| i["id"] != id);
    }
    if let Err(e) = write_data(&data).await { return e.into_response(); }
    (StatusCode::NO_CONTENT, ()).into_response()
}
