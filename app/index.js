"use strict";

/**
 * bifrost-gateway — Node.js orchestrator + browser bridge
 *
 * This process sits between the browser/WebRTC mesh and the bifrost-server
 * (Rust HTTP API). It is the "rapid iteration layer" of the DRCF architecture:
 * it never touches physics, consensus, or ledger — only orchestration.
 *
 * Configuration:
 *   PORT               = HTTP port for this gateway (default: 3000)
 *   BIFROST_SERVER_URL = URL of the Rust bifrost-server (default: http://localhost:8080)
 *
 * Endpoints:
 *   GET  /             → HTML demo UI
 *   GET  /health       → health + bifrost-server status
 *   ANY  /api/*        → transparent proxy to bifrost-server
 *   WS   /ws           → real-time tick broadcast channel
 */

const http       = require("http");
const express    = require("express");
const WebSocket  = require("ws");

const app    = express();
const server = http.createServer(app);
const wss    = new WebSocket.Server({ server });

app.use(express.json());

const PORT          = parseInt(process.env.PORT || "3000", 10);
const BIFROST_URL   = (process.env.BIFROST_SERVER_URL || "http://localhost:8080").replace(/\/$/, "");

// ─── WebSocket broadcast ──────────────────────────────────────────────────────

function broadcast(event, data) {
    const msg = JSON.stringify({ event, data, ts: Date.now() });
    wss.clients.forEach((ws) => {
        if (ws.readyState === WebSocket.OPEN) ws.send(msg);
    });
}

wss.on("connection", (ws) => {
    ws.send(JSON.stringify({ event: "connected", data: { bifrost: BIFROST_URL }, ts: Date.now() }));
});

// ─── Proxy helper ─────────────────────────────────────────────────────────────

async function bifrostFetch(path, options = {}) {
    const url = `${BIFROST_URL}${path}`;
    // Node 18+ has global fetch; fall back to http.request for Node 16
    if (typeof globalThis.fetch === "function") {
        const res = await globalThis.fetch(url, options);
        const body = await res.json();
        return { ok: res.ok, status: res.status, body };
    }
    return new Promise((resolve, reject) => {
        const u       = new URL(url);
        const method  = (options.method || "GET").toUpperCase();
        const payload = options.body ? Buffer.from(options.body) : null;
        const req     = http.request({
            hostname: u.hostname,
            port:     u.port || 8080,
            path:     u.pathname + u.search,
            method,
            headers: {
                "Content-Type": "application/json",
                ...(payload ? { "Content-Length": payload.length } : {}),
            },
        }, (res) => {
            let raw = "";
            res.on("data", (c) => (raw += c));
            res.on("end", () => {
                try {
                    resolve({ ok: res.statusCode < 400, status: res.statusCode, body: JSON.parse(raw) });
                } catch {
                    resolve({ ok: false, status: res.statusCode, body: { error: raw } });
                }
            });
        });
        req.on("error", reject);
        if (payload) req.write(payload);
        req.end();
    });
}

// ─── Health ───────────────────────────────────────────────────────────────────

app.get("/health", async (req, res) => {
    let bifrostStatus = "unreachable";
    let bifrostData   = null;
    try {
        const r = await bifrostFetch("/health");
        if (r.ok) { bifrostStatus = "ok"; bifrostData = r.body; }
    } catch (_) {}
    res.json({
        gateway:     "ok",
        bifrost:     bifrostStatus,
        bifrostData,
        wsClients:   wss.clients.size,
        bifrostUrl:  BIFROST_URL,
    });
});

// ─── API proxy ────────────────────────────────────────────────────────────────

app.all("/api/*", async (req, res) => {
    const path = req.originalUrl.replace(/^\/api/, "");
    const hasBody = ["POST", "PUT", "PATCH"].includes(req.method) && req.body;
    try {
        const r = await bifrostFetch(path, {
            method: req.method,
            body:   hasBody ? JSON.stringify(req.body) : undefined,
        });
        // Broadcast tick events to WebSocket clients
        if (req.method === "POST" && path === "/tick/advance" && r.ok) {
            broadcast("tick_advance", r.body);
        }
        if (req.method === "POST" && path === "/demo" && r.ok) {
            broadcast("demo_complete", r.body);
        }
        res.status(r.status).json(r.body);
    } catch (err) {
        res.status(503).json({ error: "bifrost-server unreachable", detail: err.message });
    }
});

// ─── Demo UI ──────────────────────────────────────────────────────────────────

app.get("/", (req, res) => {
    res.setHeader("Content-Type", "text/html; charset=utf-8");
    res.send(DEMO_HTML);
});

// ─── Start ────────────────────────────────────────────────────────────────────

server.listen(PORT, () => {
    console.log(`bifrost-gateway listening on http://0.0.0.0:${PORT}`);
    console.log(`  bifrost-server: ${BIFROST_URL}`);
    console.log(`  WebSocket:      ws://0.0.0.0:${PORT}/ws`);
});

// ─── Demo HTML ────────────────────────────────────────────────────────────────

const DEMO_HTML = /* html */`<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Bifrost Layer — Demo</title>
<style>
  :root { --bg:#0d1117; --panel:#161b22; --border:#30363d; --text:#e6edf3; --accent:#58a6ff; --green:#3fb950; --red:#f85149; --yellow:#d29922; }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { background: var(--bg); color: var(--text); font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", monospace; font-size: 14px; }
  header { background: var(--panel); border-bottom: 1px solid var(--border); padding: 16px 24px; display: flex; align-items: center; gap: 12px; }
  header h1 { font-size: 18px; font-weight: 600; color: var(--accent); }
  header .badge { background: var(--green); color: #000; font-size: 11px; font-weight: 700; padding: 2px 8px; border-radius: 12px; }
  main { padding: 24px; max-width: 1100px; margin: 0 auto; display: grid; gap: 16px; }
  .card { background: var(--panel); border: 1px solid var(--border); border-radius: 8px; overflow: hidden; }
  .card-header { padding: 12px 16px; border-bottom: 1px solid var(--border); display: flex; align-items: center; justify-content: space-between; }
  .card-header h2 { font-size: 14px; font-weight: 600; }
  .card-body { padding: 16px; }
  button { background: var(--accent); color: #000; border: none; padding: 8px 16px; border-radius: 6px; font-size: 13px; font-weight: 600; cursor: pointer; }
  button:hover { opacity: 0.85; }
  button:disabled { opacity: 0.4; cursor: not-allowed; }
  button.danger { background: var(--red); color: #fff; }
  pre { background: #010409; border: 1px solid var(--border); border-radius: 6px; padding: 12px; font-size: 12px; overflow-x: auto; white-space: pre-wrap; line-height: 1.6; max-height: 400px; overflow-y: auto; }
  .grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  .stat-row { display: flex; justify-content: space-between; padding: 6px 0; border-bottom: 1px solid var(--border); }
  .stat-row:last-child { border-bottom: none; }
  .stat-val { color: var(--accent); font-weight: 600; }
  .step { padding: 4px 8px; margin: 2px 0; background: #010409; border-left: 3px solid var(--green); border-radius: 0 4px 4px 0; font-size: 12px; }
  .ws-indicator { width: 8px; height: 8px; border-radius: 50%; background: var(--red); display: inline-block; margin-right: 6px; }
  .ws-indicator.connected { background: var(--green); }
  .log-entry { padding: 3px 0; border-bottom: 1px solid #21262d; font-size: 12px; }
  .log-entry .ts { color: #8b949e; margin-right: 8px; }
  .tag { font-size: 11px; padding: 1px 6px; border-radius: 4px; font-weight: 600; }
  .tag.accepted { background: var(--green); color: #000; }
  .tag.contested { background: var(--red); }
  .tag.pending { background: var(--yellow); color: #000; }
</style>
</head>
<body>
<header>
  <h1>⚡ Bifrost Layer</h1>
  <span class="badge">DRCF Phase 1</span>
  <span style="margin-left:auto;color:#8b949e;font-size:12px">DELPHOS decides truth — Players compute reality</span>
</header>
<main>

<div class="grid2">
  <div class="card">
    <div class="card-header">
      <h2>Server Status</h2>
      <button onclick="refreshStatus()">Refresh</button>
    </div>
    <div class="card-body" id="status-body">
      <p style="color:#8b949e">Loading…</p>
    </div>
  </div>
  <div class="card">
    <div class="card-header">
      <h2><span class="ws-indicator" id="ws-dot"></span>WebSocket</h2>
    </div>
    <div class="card-body" id="ws-log" style="max-height:180px;overflow-y:auto">
      <p style="color:#8b949e">Connecting…</p>
    </div>
  </div>
</div>

<div class="card">
  <div class="card-header">
    <h2>🚀 Full Pipeline Demo</h2>
    <button id="demo-btn" onclick="runDemo()">Run Demo</button>
  </div>
  <div class="card-body">
    <p style="color:#8b949e;margin-bottom:12px">Runs a complete isolated simulation: FILL_BOX + SIM_EXPLOSION → physics → witness quorum → tick advance.</p>
    <div id="demo-result"></div>
  </div>
</div>

<div class="card">
  <div class="card-header">
    <h2>📡 API Explorer</h2>
  </div>
  <div class="card-body">
    <div style="display:flex;gap:8px;flex-wrap:wrap;margin-bottom:12px">
      <button onclick="apiCall('GET', '/health')">GET /health</button>
      <button onclick="apiCall('GET', '/state')">GET /state</button>
      <button onclick="apiCall('GET', '/tick')">GET /tick</button>
      <button onclick="apiCall('GET', '/world/state')">GET /world/state</button>
      <button onclick="apiCall('POST', '/world/instruction', {epoch:0,payload:{op:'SetVoxel',position:{x:0,y:0,z:0},material:1}})">SetVoxel(0,0,0)</button>
    </div>
    <pre id="api-result">// Click a button to call the API</pre>
  </div>
</div>

</main>
<script>
const GATEWAY = window.location.origin;
let ws = null;

// ── WebSocket ──────────────────────────────────────────────────────────────

function connectWs() {
  const wsUrl = GATEWAY.replace(/^http/, 'ws') + '/ws';
  ws = new WebSocket(wsUrl);
  ws.onopen  = () => { setWsDot(true);  wsLog('connected to ' + wsUrl); };
  ws.onclose = () => { setWsDot(false); wsLog('disconnected — reconnecting in 3s'); setTimeout(connectWs, 3000); };
  ws.onerror = () => { setWsDot(false); };
  ws.onmessage = (e) => {
    const msg = JSON.parse(e.data);
    wsLog(msg.event + ': ' + JSON.stringify(msg.data).slice(0,80));
  };
}

function setWsDot(on) {
  const d = document.getElementById('ws-dot');
  d.classList.toggle('connected', on);
}

function wsLog(text) {
  const el = document.getElementById('ws-log');
  const ts = new Date().toLocaleTimeString();
  el.innerHTML = \`<div class="log-entry"><span class="ts">\${ts}</span>\${text}</div>\` + el.innerHTML;
}

// ── Status ─────────────────────────────────────────────────────────────────

async function refreshStatus() {
  const r = await fetch('/health');
  const d = await r.json();
  const el = document.getElementById('status-body');
  el.innerHTML = \`
    <div class="stat-row"><span>Gateway</span><span class="stat-val tag accepted">OK</span></div>
    <div class="stat-row"><span>bifrost-server</span><span class="stat-val \${d.bifrost==='ok' ? 'tag accepted' : 'tag contested'}">\${d.bifrost.toUpperCase()}</span></div>
    <div class="stat-row"><span>WS clients</span><span class="stat-val">\${d.wsClients}</span></div>
    <div class="stat-row"><span>Bifrost URL</span><span class="stat-val" style="font-size:11px">\${d.bifrostUrl}</span></div>
    \${d.bifrostData ? \`
    <div class="stat-row"><span>World tick</span><span class="stat-val">\${d.bifrostData.tick}</span></div>
    <div class="stat-row"><span>Voxels</span><span class="stat-val">\${d.bifrostData.voxels}</span></div>
    <div class="stat-row"><span>Peers</span><span class="stat-val">\${d.bifrostData.peers}</span></div>
    \` : ''}
  \`;
}

// ── Demo ───────────────────────────────────────────────────────────────────

async function runDemo() {
  const btn = document.getElementById('demo-btn');
  btn.disabled = true;
  btn.textContent = 'Running…';
  const el = document.getElementById('demo-result');
  el.innerHTML = '<p style="color:#8b949e">Executing pipeline…</p>';
  try {
    const r = await fetch('/api/demo', { method: 'POST' });
    const d = await r.json();
    if (!r.ok) { el.innerHTML = \`<pre style="border-color:var(--red)">\${JSON.stringify(d,null,2)}</pre>\`; return; }
    el.innerHTML = \`
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px;margin-bottom:12px">
        <div>
          <div class="stat-row"><span>Consensus</span><span class="tag \${d.consensus}">\${d.consensus.toUpperCase()}</span></div>
          <div class="stat-row"><span>Tick advanced</span><span class="stat-val">\${d.tick_advanced ? '✓ yes' : '✗ no'}</span></div>
          <div class="stat-row"><span>New tick</span><span class="stat-val">\${d.new_tick}</span></div>
          <div class="stat-row"><span>Peers</span><span class="stat-val">\${d.peers}</span></div>
          <div class="stat-row"><span>Instructions</span><span class="stat-val">\${d.instructions}</span></div>
          <div class="stat-row"><span>Voxels (before→after)</span><span class="stat-val">\${d.voxels_before}→\${d.voxels_after}</span></div>
          <div class="stat-row"><span>State hash</span><span class="stat-val" style="font-size:11px">\${d.state_hash.slice(0,12)}…</span></div>
        </div>
        <div>
          <p style="font-weight:600;margin-bottom:6px">Pipeline steps:</p>
          \${d.steps.map(s => \`<div class="step">\${s}</div>\`).join('')}
        </div>
      </div>
    \`;
  } catch(e) {
    el.innerHTML = \`<pre style="border-color:var(--red)">Error: \${e.message}</pre>\`;
  } finally {
    btn.disabled = false;
    btn.textContent = 'Run Demo';
    refreshStatus();
  }
}

// ── API explorer ───────────────────────────────────────────────────────────

async function apiCall(method, path, body) {
  const el = document.getElementById('api-result');
  el.textContent = '// loading…';
  try {
    const r = await fetch('/api' + path, {
      method,
      headers: body ? { 'Content-Type': 'application/json' } : {},
      body: body ? JSON.stringify(body) : undefined,
    });
    const d = await r.json();
    el.textContent = \`// \${method} \${path}  →  \${r.status}\\n\${JSON.stringify(d, null, 2)}\`;
  } catch(e) {
    el.textContent = \`// Error: \${e.message}\`;
  }
}

// ── Init ───────────────────────────────────────────────────────────────────

connectWs();
refreshStatus();
</script>
</body>
</html>`;
