//! Async HTTP API client for `/admin-api/` endpoints.
//!
//! All functions are `async` and use `gloo_net::http::Request`.
//! Call them inside `wasm_bindgen_futures::spawn_local` or Yew's
//! `use_effect_with` hook.

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

use crate::types::*;

const BASE: &str = "/admin-api";

// ─── Helper ───────────────────────────────────────────────────────────────────

async fn get<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, String> {
    let resp = Request::get(path)
        .send().await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("GET {path} → {}", resp.status()));
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

async fn post<B: Serialize, T: for<'de> Deserialize<'de>>(path: &str, body: &B) -> Result<T, String> {
    let resp = Request::post(path)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send().await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("POST {path} → {}", resp.status()));
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

async fn put<B: Serialize, T: for<'de> Deserialize<'de>>(path: &str, body: &B) -> Result<T, String> {
    let resp = Request::put(path)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send().await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("PUT {path} → {}", resp.status()));
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

async fn delete(path: &str) -> Result<(), String> {
    let resp = Request::delete(path)
        .send().await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("DELETE {path} → {}", resp.status()));
    }
    Ok(())
}

// ─── World ────────────────────────────────────────────────────────────────────

pub async fn get_world() -> Result<WorldSettings, String> {
    get(&format!("{BASE}/world")).await
}

pub async fn save_world(w: &WorldSettings) -> Result<WorldSettings, String> {
    put(&format!("{BASE}/world"), w).await
}

// ─── Biomes ───────────────────────────────────────────────────────────────────

pub async fn get_biomes() -> Result<Vec<Biome>, String> {
    get(&format!("{BASE}/biomes")).await
}

pub async fn create_biome(b: &Biome) -> Result<Biome, String> {
    post(&format!("{BASE}/biomes"), b).await
}

pub async fn update_biome(b: &Biome) -> Result<Biome, String> {
    put(&format!("{BASE}/biomes/{}", b.id), b).await
}

pub async fn delete_biome(id: &str) -> Result<(), String> {
    delete(&format!("{BASE}/biomes/{id}")).await
}

// ─── Story ────────────────────────────────────────────────────────────────────

pub async fn get_story() -> Result<StoryData, String> {
    get(&format!("{BASE}/story")).await
}

pub async fn create_arc(arc: &StoryArc) -> Result<StoryArc, String> {
    post(&format!("{BASE}/story/arcs"), arc).await
}

pub async fn update_arc(arc: &StoryArc) -> Result<StoryArc, String> {
    put(&format!("{BASE}/story/arcs/{}", arc.id), arc).await
}

pub async fn delete_arc(id: &str) -> Result<(), String> {
    delete(&format!("{BASE}/story/arcs/{id}")).await
}

pub async fn create_beat(arc_id: &str, beat: &StoryBeat) -> Result<StoryBeat, String> {
    post(&format!("{BASE}/story/arcs/{arc_id}/beats"), beat).await
}

pub async fn update_beat(arc_id: &str, beat: &StoryBeat) -> Result<StoryBeat, String> {
    put(&format!("{BASE}/story/arcs/{arc_id}/beats/{}", beat.id), beat).await
}

pub async fn delete_beat(arc_id: &str, beat_id: &str) -> Result<(), String> {
    delete(&format!("{BASE}/story/arcs/{arc_id}/beats/{beat_id}")).await
}

// ─── NPCs ─────────────────────────────────────────────────────────────────────

pub async fn get_npcs() -> Result<Vec<Npc>, String> {
    get(&format!("{BASE}/npcs")).await
}

pub async fn create_npc(n: &Npc) -> Result<Npc, String> {
    post(&format!("{BASE}/npcs"), n).await
}

pub async fn update_npc(n: &Npc) -> Result<Npc, String> {
    put(&format!("{BASE}/npcs/{}", n.id), n).await
}

pub async fn delete_npc(id: &str) -> Result<(), String> {
    delete(&format!("{BASE}/npcs/{id}")).await
}

// ─── Quests ───────────────────────────────────────────────────────────────────

pub async fn get_quests() -> Result<Vec<Quest>, String> {
    get(&format!("{BASE}/quests")).await
}

pub async fn create_quest(q: &Quest) -> Result<Quest, String> {
    post(&format!("{BASE}/quests"), q).await
}

pub async fn update_quest(q: &Quest) -> Result<Quest, String> {
    put(&format!("{BASE}/quests/{}", q.id), q).await
}

pub async fn delete_quest(id: &str) -> Result<(), String> {
    delete(&format!("{BASE}/quests/{id}")).await
}

// ─── Loot — Monsters ─────────────────────────────────────────────────────────

pub async fn get_monsters() -> Result<Vec<Monster>, String> {
    get(&format!("{BASE}/loot/monsters")).await
}

pub async fn create_monster(m: &Monster) -> Result<Monster, String> {
    post(&format!("{BASE}/loot/monsters"), m).await
}

pub async fn update_monster(m: &Monster) -> Result<Monster, String> {
    put(&format!("{BASE}/loot/monsters/{}", m.id), m).await
}

pub async fn delete_monster(id: &str) -> Result<(), String> {
    delete(&format!("{BASE}/loot/monsters/{id}")).await
}

// ─── Loot — Items ────────────────────────────────────────────────────────────

pub async fn get_items() -> Result<Vec<LootItem>, String> {
    get(&format!("{BASE}/loot/items")).await
}

pub async fn create_item(i: &LootItem) -> Result<LootItem, String> {
    post(&format!("{BASE}/loot/items"), i).await
}

pub async fn update_item(i: &LootItem) -> Result<LootItem, String> {
    put(&format!("{BASE}/loot/items/{}", i.id), i).await
}

pub async fn delete_item(id: &str) -> Result<(), String> {
    delete(&format!("{BASE}/loot/items/{id}")).await
}
