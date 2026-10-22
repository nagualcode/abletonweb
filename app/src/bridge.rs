//! The bridge server (`live-remote serve`): keeps a TCP link to the LiveWeb
//! remote script, serves the embedded web app and relays state, meter ticks
//! and commands over WebSocket.

use crate::config::Config;
use crate::diff::diff;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use include_dir::{Dir, include_dir};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot};

static WEB: Dir = include_dir!("$CARGO_MANIFEST_DIR/../web/dist");

const RECONNECT: Duration = Duration::from_secs(1);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

type Reply = Result<Value, String>;

struct LiveRequest {
    cmd: String,
    args: Value,
    reply: oneshot::Sender<Reply>,
}

struct Shared {
    config: Config,
    /// Live version while the remote script is connected.
    live: RwLock<Option<String>>,
    /// Last state sent to clients; patches are computed against it.
    state: RwLock<Option<Value>>,
    /// Serialized messages for every client: live / state / patch / tick.
    events: broadcast::Sender<Arc<str>>,
    requests: mpsc::Sender<LiveRequest>,
    clients: AtomicUsize,
}

pub fn serve(config: Config) -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    runtime.block_on(run(config))
}

async fn run(config: Config) -> Result<(), String> {
    let (events, _) = broadcast::channel(256);
    let (requests, requests_rx) = mpsc::channel(64);
    let shared = Arc::new(Shared {
        config: config.clone(),
        live: RwLock::new(None),
        state: RwLock::new(None),
        events,
        requests,
        clients: AtomicUsize::new(0),
    });
    tokio::spawn(live_link(shared.clone(), requests_rx));

    let app = axum::Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_upgrade))
        .fallback(get(static_file))
        .with_state(shared);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port))
        .await
        .map_err(|e| format!("cannot listen on port {}: {e}", config.port))?;
    println!(
        "live-remote bridge on port {} (token {})",
        config.port,
        if config.require_token { "required" } else { "off" }
    );
    axum::serve(listener, app).await.map_err(|e| e.to_string())
}

fn broadcast(shared: &Shared, msg: &Value) {
    let _ = shared.events.send(Arc::from(msg.to_string()));
}

// ------------------------------------------------------------------ Live

async fn live_link(shared: Arc<Shared>, mut requests: mpsc::Receiver<LiveRequest>) {
    loop {
        match TcpStream::connect(("127.0.0.1", shared.config.live_port)).await {
            Ok(stream) => {
                let _ = stream.set_nodelay(true);
                live_session(&shared, stream, &mut requests).await;
                let was_connected = shared.live.write().unwrap().take().is_some();
                *shared.state.write().unwrap() = None;
                if was_connected {
                    println!("[live] disconnected, retrying...");
                    broadcast(&shared, &json!({"t": "live", "connected": false}));
                }
            }
            Err(_) => {
                // Not connected: fail requests right away instead of timing out.
                while let Ok(req) = requests.try_recv() {
                    let _ = req.reply.send(Err("Live is not connected".into()));
                }
            }
        }
        tokio::time::sleep(RECONNECT).await;
    }
}

async fn live_session(shared: &Shared, stream: TcpStream, requests: &mut mpsc::Receiver<LiveRequest>) {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read).lines();
    let mut pending: HashMap<u64, oneshot::Sender<Reply>> = HashMap::new();
    let mut next_id = 1u64;
    loop {
        tokio::select! {
            line = lines.next_line() => {
                let Ok(Some(line)) = line else { break };
                handle_live_line(shared, &line, &mut pending);
            }
            Some(req) = requests.recv() => {
                if shared.live.read().unwrap().is_none() {
                    let _ = req.reply.send(Err("Live is not connected".into()));
                    continue;
                }
                let id = next_id;
                next_id += 1;
                let msg = json!({"id": id, "cmd": req.cmd, "args": req.args}).to_string() + "\n";
                if write.write_all(msg.as_bytes()).await.is_err() {
                    let _ = req.reply.send(Err("Live disconnected".into()));
                    break;
                }
                pending.insert(id, req.reply);
            }
        }
    }
    for (_, reply) in pending.drain() {
        let _ = reply.send(Err("Live disconnected".into()));
    }
}

fn handle_live_line(shared: &Shared, line: &str, pending: &mut HashMap<u64, oneshot::Sender<Reply>>) {
    // Ticks are the bulk of the traffic and already in client format.
    if line.starts_with(r#"{"t":"tick""#) {
        let _ = shared.events.send(Arc::from(line));
        return;
    }
    let Ok(msg) = serde_json::from_str::<Value>(line) else {
        eprintln!("[live] invalid json: {}", &line[..line.len().min(200)]);
        return;
    };
    match msg["t"].as_str() {
        Some("hello") => {
            let version = msg["live"].as_str().unwrap_or("").to_string();
            println!("[live] connected (Live {version})");
            *shared.live.write().unwrap() = Some(version.clone());
            broadcast(shared, &json!({"t": "live", "connected": true, "version": version}));
        }
        Some("state") => {
            let next = msg["state"].clone();
            let mut last = shared.state.write().unwrap();
            match last.as_ref() {
                None => broadcast(shared, &json!({"t": "state", "state": next})),
                Some(prev) => {
                    let ops = diff(prev, &next);
                    if !ops.is_empty() {
                        broadcast(shared, &json!({"t": "patch", "ops": ops}));
                    }
                }
            }
            *last = Some(next);
        }
        Some("tick") => {
            let _ = shared.events.send(Arc::from(line));
        }
        Some("reply") => {
            let Some(reply) = msg["id"].as_u64().and_then(|id| pending.remove(&id)) else { return };
            let result = if msg["ok"].as_bool() == Some(true) {
                Ok(msg["result"].clone())
            } else {
                Err(msg["error"].as_str().unwrap_or("error").to_string())
            };
            let _ = reply.send(result);
        }
        _ => {}
    }
}

async fn live_request(shared: &Shared, cmd: String, args: Value) -> Reply {
    let (tx, rx) = oneshot::channel();
    shared
        .requests
        .send(LiveRequest { cmd, args, reply: tx })
        .await
        .map_err(|_| "bridge stopping".to_string())?;
    match tokio::time::timeout(REQUEST_TIMEOUT, rx).await {
        Ok(Ok(reply)) => reply,
        Ok(Err(_)) => Err("Live disconnected".into()),
        Err(_) => Err("timeout waiting for Live".into()),
    }
}

// ------------------------------------------------------------------ HTTP

async fn health(State(shared): State<Arc<Shared>>) -> impl IntoResponse {
    let live = shared.live.read().unwrap().clone();
    axum::Json(json!({
        "live": live.is_some(),
        "version": live.unwrap_or_default(),
        "clients": shared.clients.load(Ordering::Relaxed),
    }))
}

async fn static_file(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    // Single-page app: unknown paths fall back to index.html.
    let file = WEB.get_file(if path.is_empty() { "index.html" } else { path })
        .or_else(|| WEB.get_file("index.html"));
    let Some(file) = file else {
        return (StatusCode::NOT_FOUND, "web app missing").into_response();
    };
    let name = file.path().to_string_lossy();
    let ext = name.rsplit('.').next().unwrap_or("");
    let cache = if ext == "html" { "no-cache" } else { "public, max-age=31536000, immutable" };
    ([(header::CONTENT_TYPE, mime(ext)), (header::CACHE_CONTROL, cache)], file.contents()).into_response()
}

fn mime(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "webmanifest" => "application/manifest+json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

// ------------------------------------------------------------- WebSocket

async fn ws_upgrade(
    State(shared): State<Arc<Shared>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    ws: WebSocketUpgrade,
) -> Response {
    if !allowed(&shared.config, &headers, query.get("t")) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    ws.max_message_size(64 * 1024).on_upgrade(move |socket| client(shared, socket))
}

/// Without a token anyone on the LAN may connect, but web pages from other
/// sites must not: browsers let any site open WebSockets to LAN addresses.
/// Requiring Origin == Host blocks that, and accepting only IP/localhost/.local
/// hosts blocks DNS-rebinding tricks around the Origin check.
fn allowed(config: &Config, headers: &HeaderMap, token: Option<&String>) -> bool {
    let header = |name| headers.get(name).and_then(|v| v.to_str().ok()).map(str::to_ascii_lowercase);
    let host = header(header::HOST).unwrap_or_default();
    if let Some(origin) = header(header::ORIGIN) {
        let origin_host = origin.split("://").nth(1).unwrap_or("");
        if origin_host != host {
            return false;
        }
    }
    let hostname = host.rsplit_once(':').map_or(host.as_str(), |(h, port)| {
        if port.chars().all(|c| c.is_ascii_digit()) { h } else { host.as_str() }
    });
    let hostname = hostname.trim_start_matches('[').trim_end_matches(']');
    let local = hostname == "localhost" || hostname.ends_with(".local") || hostname.parse::<IpAddr>().is_ok();
    local && (!config.require_token || token == config.token.as_ref())
}

async fn client(shared: Arc<Shared>, socket: WebSocket) {
    let count = shared.clients.fetch_add(1, Ordering::Relaxed) + 1;
    println!("[web] client connected ({count} total)");
    let (mut sink, mut stream) = socket.split();
    // Subscribe before sending the snapshot so no patch falls in between
    // (replaying one already in the snapshot is harmless: ops are replaces).
    let mut events = shared.events.subscribe();
    let (replies_tx, mut replies) = mpsc::channel::<String>(32);

    let hello = {
        let live = shared.live.read().unwrap().clone();
        let mut msgs = vec![json!({"t": "live", "connected": live.is_some(), "version": live}).to_string()];
        if let Some(state) = shared.state.read().unwrap().as_ref() {
            msgs.push(json!({"t": "state", "state": state}).to_string());
        }
        msgs
    };
    let reader_shared = shared.clone();
    let reader = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            let Message::Text(text) = msg else { continue };
            let Ok(cmd) = serde_json::from_str::<Value>(text.as_str()) else { continue };
            let (Some(id), Some(name)) = (cmd["id"].as_u64(), cmd["cmd"].as_str()) else { continue };
            let (shared, replies_tx, name, args) =
                (reader_shared.clone(), replies_tx.clone(), name.to_string(), cmd["args"].clone());
            // Commands run concurrently so a slow one never blocks faders.
            tokio::spawn(async move {
                let reply = match live_request(&shared, name, args).await {
                    Ok(result) => json!({"t": "reply", "id": id, "ok": true, "result": result}),
                    Err(error) => json!({"t": "reply", "id": id, "ok": false, "error": error}),
                };
                let _ = replies_tx.send(reply.to_string()).await;
            });
        }
    });

    'send: {
        for msg in hello {
            if sink.send(Message::Text(msg.into())).await.is_err() {
                break 'send;
            }
        }
        loop {
            let msg: String = tokio::select! {
                event = events.recv() => match event {
                    Ok(msg) => msg.to_string(),
                    // Too slow (bad Wi-Fi): skip ahead and resync with a full state.
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let state = shared.state.read().unwrap().clone();
                        match state {
                            Some(state) => json!({"t": "state", "state": state}).to_string(),
                            None => continue,
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                },
                Some(reply) = replies.recv() => reply,
            };
            if reader.is_finished() || sink.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    }
    reader.abort();
    let count = shared.clients.fetch_sub(1, Ordering::Relaxed) - 1;
    println!("[web] client disconnected ({count} total)");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(host: &str, origin: Option<&str>) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::HOST, host.parse().unwrap());
        if let Some(o) = origin {
            h.insert(header::ORIGIN, o.parse().unwrap());
        }
        h
    }

    #[test]
    fn upgrade_rules() {
        let mut config = Config { token: Some("secret".into()), ..Config::default() };
        let ok = |c: &Config, host, origin, token: Option<&str>| allowed(c, &headers(host, origin), token.map(String::from).as_ref());
        assert!(ok(&config, "127.0.0.1:6090", None, None));
        assert!(ok(&config, "192.168.1.5:6090", Some("http://192.168.1.5:6090"), None));
        assert!(ok(&config, "Mac.local:6090", Some("http://mac.local:6090"), None));
        assert!(!ok(&config, "192.168.1.5:6090", Some("https://evil.example"), None));
        assert!(!ok(&config, "evil.example:6090", Some("http://evil.example:6090"), None));
        config.require_token = true;
        assert!(!ok(&config, "127.0.0.1:6090", None, None));
        assert!(!ok(&config, "127.0.0.1:6090", None, Some("wrong")));
        assert!(ok(&config, "127.0.0.1:6090", None, Some("secret")));
    }
}
