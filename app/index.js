"use strict";

/**
 * bifrost-gateway — Node.js orchestrator + browser bridge
 *
 * Endpoints:
 *   GET  /         → game.html  (NOVA World RPG)
 *   GET  /demo     → redirects to /  (fixes 405 from direct URL navigation)
 *   GET  /health   → gateway + bifrost-server status
 *   ANY  /api/*    → transparent proxy to bifrost-server
 *   WS   /ws       → real-time tick broadcast channel
 */

const http    = require("http");
const path    = require("path");
const fs      = require("fs");
const express = require("express");
const WebSocket = require("ws");

const app    = express();
const server = http.createServer(app);
const wss    = new WebSocket.Server({ server });

app.use(express.json());

const PORT        = parseInt(process.env.PORT || "3000", 10);
const BIFROST_URL = (process.env.BIFROST_SERVER_URL || "http://localhost:8080").replace(/\/$/, "");
const GAME_HTML   = path.join(__dirname, "game.html");

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

// Serve the RPG game at root
app.get("/", (req, res) => {
    res.setHeader("Content-Type", "text/html; charset=utf-8");
    res.sendFile(GAME_HTML);
});

// Fix 405: browsers navigate to /demo via GET — redirect to game
app.get("/demo", (req, res) => res.redirect("/"));

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
    console.log(`\nOpen http://localhost:${PORT} in your browser\n`);
});
