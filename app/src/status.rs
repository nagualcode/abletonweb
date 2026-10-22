//! Background polling of the bridge service, the bridge itself and Live.

use crate::config;
use crate::remote_script::{self, ScriptStatus};
use crate::service::{self, Service};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const POLL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Default)]
pub struct Health {
    pub live: bool,
    pub version: String,
    pub clients: u64,
}

#[derive(Clone, Debug)]
pub struct Status {
    pub service: Service,
    /// None when the bridge does not answer on the configured port.
    pub health: Option<Health>,
    pub remote_script: ScriptStatus,
}

impl Default for Status {
    fn default() -> Self {
        Status { service: Service::Unknown, health: None, remote_script: ScriptStatus::NotInstalled }
    }
}

pub fn spawn_poller(ctx: eframe::egui::Context) -> Arc<Mutex<Status>> {
    let shared = Arc::new(Mutex::new(Status::default()));
    let out = shared.clone();
    std::thread::spawn(move || {
        loop {
            let status = check(config::load().port);
            *out.lock().unwrap() = status;
            ctx.request_repaint();
            std::thread::sleep(POLL);
        }
    });
    shared
}

pub fn check(port: u16) -> Status {
    Status { service: service::status(), health: health(port), remote_script: remote_script::status() }
}

/// Minimal HTTP GET of the bridge's /health endpoint (no HTTP client needed).
fn health(port: u16) -> Option<Health> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(300)).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(500))).ok()?;
    write!(stream, "GET /health HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    let body = response.split("\r\n\r\n").nth(1)?;
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    Some(Health {
        live: json["live"].as_bool().unwrap_or(false),
        version: json["version"].as_str().unwrap_or("").to_string(),
        clients: json["clients"].as_u64().unwrap_or(0),
    })
}
