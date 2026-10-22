//! The bridge config at ~/.config/abletonweb/config.json.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_live_port")]
    pub live_port: u16,
    #[serde(default)]
    pub require_token: bool,
    // The bridge generates the token on its first run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    // Keys added by future versions of the bridge are preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

fn default_port() -> u16 {
    6090
}

fn default_live_port() -> u16 {
    9877
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: default_port(),
            live_port: default_live_port(),
            require_token: false,
            token: None,
            extra: Default::default(),
        }
    }
}

pub fn path() -> PathBuf {
    home().join(".config/abletonweb/config.json")
}

pub fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
}

pub fn load() -> Config {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Loads the config, creating the file (with a fresh token) when needed.
pub fn load_or_init() -> Config {
    let mut config = load();
    if config.token.is_none() || !path().exists() {
        config.token.get_or_insert_with(new_token);
        if let Err(e) = save(&config) {
            eprintln!("cannot write {}: {e}", path().display());
        }
    }
    config
}

fn new_token() -> String {
    use std::io::Read;
    let mut bytes = [0u8; 12];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .expect("read /dev/urandom");
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    // 12 bytes = 96 bits = 16 base64url characters.
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (chunk[1] as u32) << 8 | chunk[2] as u32;
        for shift in [18, 12, 6, 0] {
            out.push(ALPHABET[(n >> shift & 63) as usize] as char);
        }
    }
    out
}

pub fn save(config: &Config) -> std::io::Result<()> {
    let path = path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(config).expect("config serializes");
    std::fs::write(&path, json + "\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}
