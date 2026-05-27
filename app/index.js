"use strict";

/**
 * bifrost-gateway — Node.js orchestrator + browser bridge
 *
 * Endpoints:
 *   GET  /           → game.html  (NOVA World RPG)
 *   GET  /admin      → admin.html (World Admin Dashboard — Rust/WASM)
 *   GET  /admin/pkg/* → WASM package files
 *   GET  /demo       → redirects to /  (fixes 405 from direct URL navigation)
 *   GET  /health     → gateway + bifrost-server status
 *   ANY  /api/*      → transparent proxy to bifrost-server
 *   ANY  /admin-api/* → world data CRUD (world-data.json)
 *   WS   /ws         → real-time tick broadcast channel
 */

const http      = require("http");
const path      = require("path");
const fs        = require("fs");
const express   = require("express");
const WebSocket = require("ws");

const app    = express();
const server = http.createServer(app);
const wss    = new WebSocket.Server({ server });

app.use(express.json());

const PORT          = parseInt(process.env.PORT || "3000", 10);
const BIFROST_URL   = (process.env.BIFROST_SERVER_URL || "http://localhost:8080").replace(/\/$/, "");
const GAME_HTML     = path.join(__dirname, "game.html");
const ADMIN_HTML    = path.join(__dirname, "admin.html");
const WORLD_DATA    = path.join(__dirname, "world-data.json");
const ADMIN_PKG_DIR = path.join(__dirname, "pkg", "admin");

// ─── World data helpers ───────────────────────────────────────────────────────

function loadWorldData() {
    try {
        return JSON.parse(fs.readFileSync(WORLD_DATA, "utf8"));
    } catch (_) {
        return { world: {}, biomes: [], story: { worldMood: "calm", arcs: [] }, npcs: [], quests: [], loot: { monsters: [], items: [] } };
    }
}

function saveWorldData(data) {
    fs.writeFileSync(WORLD_DATA, JSON.stringify(data, null, 2), "utf8");
}

function genId(prefix = "item") {
    return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
}

// ─── WebSocket broadcast ──────────────────────────────────────────────────────

function broadcast(event, data) {
    const msg = JSON.stringify({ event, data, ts: Date.now() });
    wss.clients.forEach(ws => {
        if (ws.readyState === WebSocket.OPEN) ws.send(msg);
    });
}

wss.on("connection", ws => {
    ws.send(JSON.stringify({ event: "connected", data: { bifrost: BIFROST_URL }, ts: Date.now() }));
});

// ─── Proxy helper ─────────────────────────────────────────────────────────────

async function bifrostFetch(urlPath, options = {}) {
    const url = `${BIFROST_URL}${urlPath}`;
    if (typeof globalThis.fetch === "function") {
        const res  = await globalThis.fetch(url, options);
        const body = await res.json().catch(() => ({}));
        return { ok: res.ok, status: res.status, body };
    }
    return new Promise((resolve, reject) => {
        const u      = new URL(url);
        const method = (options.method || "GET").toUpperCase();
        const payload = options.body ? Buffer.from(options.body) : null;
        const req = http.request({
            hostname: u.hostname,
            port:     u.port || 8080,
            path:     u.pathname + u.search,
            method,
            headers: {
                "Content-Type": "application/json",
                ...(payload ? { "Content-Length": payload.length } : {}),
            },
        }, res => {
            let raw = "";
            res.on("data", c => raw += c);
            res.on("end", () => {
                try   { resolve({ ok: res.statusCode < 400, status: res.statusCode, body: JSON.parse(raw) }); }
                catch { resolve({ ok: false, status: res.statusCode, body: { error: raw } }); }
            });
        });
        req.on("error", reject);
        if (payload) req.write(payload);
        req.end();
    });
}

// ─── Game ─────────────────────────────────────────────────────────────────────

app.get("/", (req, res) => {
    res.setHeader("Content-Type", "text/html; charset=utf-8");
    res.sendFile(GAME_HTML);
});

app.get("/demo", (req, res) => res.redirect("/"));

// ─── Admin dashboard (WASM) ───────────────────────────────────────────────────

app.get("/admin", (req, res) => {
    if (!fs.existsSync(ADMIN_HTML)) {
        return res.status(503).send("<h2>Admin not built yet — run: wasm-pack build bifrost/admin --target web --out-dir app/pkg/admin</h2>");
    }
    res.setHeader("Content-Type", "text/html; charset=utf-8");
    res.sendFile(ADMIN_HTML);
});

// Serve the compiled WASM package (JS glue + .wasm binary)
app.use("/admin/pkg", express.static(ADMIN_PKG_DIR));

// ─── Health ───────────────────────────────────────────────────────────────────

app.get("/health", async (req, res) => {
    let bifrostStatus = "unreachable";
    let bifrostData   = null;
    try {
        const r = await bifrostFetch("/health");
        if (r.ok) { bifrostStatus = "ok"; bifrostData = r.body; }
    } catch (_) {}
    res.json({ gateway: "ok", bifrost: bifrostStatus, bifrostData, wsClients: wss.clients.size, bifrostUrl: BIFROST_URL });
});

// ─── Admin API — World data CRUD ──────────────────────────────────────────────

// GET  /admin-api/world
// PUT  /admin-api/world
app.get("/admin-api/world", (req, res) => {
    const d = loadWorldData();
    res.json(d.world);
});
app.put("/admin-api/world", (req, res) => {
    const d = loadWorldData();
    d.world = { ...d.world, ...req.body };
    saveWorldData(d);
    broadcast("world_updated", d.world);
    res.json(d.world);
});

// ─── Biomes ────────────────────────────────────────────────────────────────────

app.get("/admin-api/biomes", (req, res) => {
    res.json(loadWorldData().biomes);
});
app.post("/admin-api/biomes", (req, res) => {
    const d    = loadWorldData();
    const item = { id: genId("biome"), ...req.body };
    d.biomes.push(item);
    saveWorldData(d);
    broadcast("biomes_updated", d.biomes);
    res.status(201).json(item);
});
app.put("/admin-api/biomes/:id", (req, res) => {
    const d   = loadWorldData();
    const idx = d.biomes.findIndex(b => b.id === req.params.id);
    if (idx === -1) return res.status(404).json({ error: "not found" });
    d.biomes[idx] = { ...d.biomes[idx], ...req.body, id: req.params.id };
    saveWorldData(d);
    broadcast("biomes_updated", d.biomes);
    res.json(d.biomes[idx]);
});
app.delete("/admin-api/biomes/:id", (req, res) => {
    const d = loadWorldData();
    d.biomes = d.biomes.filter(b => b.id !== req.params.id);
    saveWorldData(d);
    broadcast("biomes_updated", d.biomes);
    res.json({ deleted: req.params.id });
});

// ─── Story ─────────────────────────────────────────────────────────────────────

app.get("/admin-api/story", (req, res) => {
    res.json(loadWorldData().story);
});
app.put("/admin-api/story/mood", (req, res) => {
    const d = loadWorldData();
    d.story.worldMood = req.body.worldMood || d.story.worldMood;
    saveWorldData(d);
    broadcast("story_updated", d.story);
    res.json({ worldMood: d.story.worldMood });
});
// Arcs
app.get("/admin-api/story/arcs", (req, res) => {
    res.json(loadWorldData().story.arcs);
});
app.post("/admin-api/story/arcs", (req, res) => {
    const d    = loadWorldData();
    const item = { id: genId("arc"), beats: [], ...req.body };
    d.story.arcs.push(item);
    saveWorldData(d);
    broadcast("story_updated", d.story);
    res.status(201).json(item);
});
app.put("/admin-api/story/arcs/:id", (req, res) => {
    const d   = loadWorldData();
    const idx = d.story.arcs.findIndex(a => a.id === req.params.id);
    if (idx === -1) return res.status(404).json({ error: "not found" });
    d.story.arcs[idx] = { ...d.story.arcs[idx], ...req.body, id: req.params.id };
    saveWorldData(d);
    broadcast("story_updated", d.story);
    res.json(d.story.arcs[idx]);
});
app.delete("/admin-api/story/arcs/:id", (req, res) => {
    const d = loadWorldData();
    d.story.arcs = d.story.arcs.filter(a => a.id !== req.params.id);
    saveWorldData(d);
    broadcast("story_updated", d.story);
    res.json({ deleted: req.params.id });
});
// Beats within an arc
app.post("/admin-api/story/arcs/:arcId/beats", (req, res) => {
    const d   = loadWorldData();
    const arc = d.story.arcs.find(a => a.id === req.params.arcId);
    if (!arc) return res.status(404).json({ error: "arc not found" });
    const beat = { id: genId("beat"), ...req.body };
    arc.beats.push(beat);
    saveWorldData(d);
    broadcast("story_updated", d.story);
    res.status(201).json(beat);
});
app.put("/admin-api/story/arcs/:arcId/beats/:beatId", (req, res) => {
    const d   = loadWorldData();
    const arc = d.story.arcs.find(a => a.id === req.params.arcId);
    if (!arc) return res.status(404).json({ error: "arc not found" });
    const idx = arc.beats.findIndex(b => b.id === req.params.beatId);
    if (idx === -1) return res.status(404).json({ error: "beat not found" });
    arc.beats[idx] = { ...arc.beats[idx], ...req.body, id: req.params.beatId };
    saveWorldData(d);
    broadcast("story_updated", d.story);
    res.json(arc.beats[idx]);
});
app.delete("/admin-api/story/arcs/:arcId/beats/:beatId", (req, res) => {
    const d   = loadWorldData();
    const arc = d.story.arcs.find(a => a.id === req.params.arcId);
    if (!arc) return res.status(404).json({ error: "arc not found" });
    arc.beats = arc.beats.filter(b => b.id !== req.params.beatId);
    saveWorldData(d);
    broadcast("story_updated", d.story);
    res.json({ deleted: req.params.beatId });
});

// ─── NPCs ─────────────────────────────────────────────────────────────────────

app.get("/admin-api/npcs", (req, res) => {
    res.json(loadWorldData().npcs);
});
app.post("/admin-api/npcs", (req, res) => {
    const d    = loadWorldData();
    const item = { id: genId("npc"), lines: [], cooldownMs: 5000, ...req.body };
    d.npcs.push(item);
    saveWorldData(d);
    broadcast("npcs_updated", d.npcs);
    res.status(201).json(item);
});
app.put("/admin-api/npcs/:id", (req, res) => {
    const d   = loadWorldData();
    const idx = d.npcs.findIndex(n => n.id === req.params.id);
    if (idx === -1) return res.status(404).json({ error: "not found" });
    d.npcs[idx] = { ...d.npcs[idx], ...req.body, id: req.params.id };
    saveWorldData(d);
    broadcast("npcs_updated", d.npcs);
    res.json(d.npcs[idx]);
});
app.delete("/admin-api/npcs/:id", (req, res) => {
    const d = loadWorldData();
    d.npcs = d.npcs.filter(n => n.id !== req.params.id);
    saveWorldData(d);
    broadcast("npcs_updated", d.npcs);
    res.json({ deleted: req.params.id });
});

// ─── Quests ───────────────────────────────────────────────────────────────────

app.get("/admin-api/quests", (req, res) => {
    res.json(loadWorldData().quests);
});
app.post("/admin-api/quests", (req, res) => {
    const d    = loadWorldData();
    const item = { id: genId("quest"), ...req.body };
    d.quests.push(item);
    saveWorldData(d);
    broadcast("quests_updated", d.quests);
    res.status(201).json(item);
});
app.put("/admin-api/quests/:id", (req, res) => {
    const d   = loadWorldData();
    const idx = d.quests.findIndex(q => q.id === req.params.id);
    if (idx === -1) return res.status(404).json({ error: "not found" });
    d.quests[idx] = { ...d.quests[idx], ...req.body, id: req.params.id };
    saveWorldData(d);
    broadcast("quests_updated", d.quests);
    res.json(d.quests[idx]);
});
app.delete("/admin-api/quests/:id", (req, res) => {
    const d = loadWorldData();
    d.quests = d.quests.filter(q => q.id !== req.params.id);
    saveWorldData(d);
    broadcast("quests_updated", d.quests);
    res.json({ deleted: req.params.id });
});

// ─── Loot — Monsters ──────────────────────────────────────────────────────────

app.get("/admin-api/loot/monsters", (req, res) => {
    res.json(loadWorldData().loot.monsters);
});
app.post("/admin-api/loot/monsters", (req, res) => {
    const d    = loadWorldData();
    const item = { id: genId("monster"), drops: [], ...req.body };
    d.loot.monsters.push(item);
    saveWorldData(d);
    broadcast("loot_updated", d.loot);
    res.status(201).json(item);
});
app.put("/admin-api/loot/monsters/:id", (req, res) => {
    const d   = loadWorldData();
    const idx = d.loot.monsters.findIndex(m => m.id === req.params.id);
    if (idx === -1) return res.status(404).json({ error: "not found" });
    d.loot.monsters[idx] = { ...d.loot.monsters[idx], ...req.body, id: req.params.id };
    saveWorldData(d);
    broadcast("loot_updated", d.loot);
    res.json(d.loot.monsters[idx]);
});
app.delete("/admin-api/loot/monsters/:id", (req, res) => {
    const d = loadWorldData();
    d.loot.monsters = d.loot.monsters.filter(m => m.id !== req.params.id);
    saveWorldData(d);
    broadcast("loot_updated", d.loot);
    res.json({ deleted: req.params.id });
});

// ─── Loot — Items ─────────────────────────────────────────────────────────────

app.get("/admin-api/loot/items", (req, res) => {
    res.json(loadWorldData().loot.items);
});
app.post("/admin-api/loot/items", (req, res) => {
    const d    = loadWorldData();
    const item = { id: genId("item"), ...req.body };
    d.loot.items.push(item);
    saveWorldData(d);
    broadcast("loot_updated", d.loot);
    res.status(201).json(item);
});
app.put("/admin-api/loot/items/:id", (req, res) => {
    const d   = loadWorldData();
    const idx = d.loot.items.findIndex(i => i.id === req.params.id);
    if (idx === -1) return res.status(404).json({ error: "not found" });
    d.loot.items[idx] = { ...d.loot.items[idx], ...req.body, id: req.params.id };
    saveWorldData(d);
    broadcast("loot_updated", d.loot);
    res.json(d.loot.items[idx]);
});
app.delete("/admin-api/loot/items/:id", (req, res) => {
    const d = loadWorldData();
    d.loot.items = d.loot.items.filter(i => i.id !== req.params.id);
    saveWorldData(d);
    broadcast("loot_updated", d.loot);
    res.json({ deleted: req.params.id });
});

// ─── API proxy ────────────────────────────────────────────────────────────────

app.all("/api/*", async (req, res) => {
    const apiPath = req.originalUrl.replace(/^\/api/, "");
    const hasBody = ["POST","PUT","PATCH"].includes(req.method) && req.body;
    try {
        const r = await bifrostFetch(apiPath, {
            method: req.method,
            body:   hasBody ? JSON.stringify(req.body) : undefined,
        });
        if (req.method === "POST" && apiPath === "/tick/advance" && r.ok) broadcast("tick_advance", r.body);
        if (req.method === "POST" && apiPath === "/demo"         && r.ok) broadcast("demo_complete", r.body);
        res.status(r.status).json(r.body);
    } catch (err) {
        res.status(503).json({ error: "bifrost-server unreachable", detail: err.message });
    }
});

// ─── Start ────────────────────────────────────────────────────────────────────

server.listen(PORT, "0.0.0.0", () => {
    console.log(`\nbifrost-gateway  http://0.0.0.0:${PORT}`);
    console.log(`bifrost-server   ${BIFROST_URL}`);
    console.log(`WebSocket        ws://0.0.0.0:${PORT}/ws`);
    console.log(`Admin dashboard  http://localhost:${PORT}/admin`);
    console.log(`\nOpen http://localhost:${PORT} in your browser\n`);
});
