//! Installs the embedded LiveWeb remote script into Live's User Library.

use crate::config;
use include_dir::{Dir, include_dir};
use std::path::PathBuf;

static SCRIPT: Dir = include_dir!("$CARGO_MANIFEST_DIR/../remote_script/LiveWeb");

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScriptStatus {
    NotInstalled,
    Outdated,
    Installed,
}

/// Live's User Library, read from the newest Live's Library.cfg (users can
/// move it); defaults to ~/Music/Ableton/User Library.
pub fn user_library() -> PathBuf {
    let prefs = config::home().join("Library/Preferences/Ableton");
    let newest_cfg = std::fs::read_dir(&prefs)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("Live "))
        .map(|e| e.path().join("Library.cfg"))
        .filter_map(|p| Some((p.metadata().ok()?.modified().ok()?, p)))
        .max();
    newest_cfg
        .and_then(|(_, cfg)| std::fs::read_to_string(cfg).ok())
        .and_then(|text| {
            let section = &text[text.find("<UserLibrary>")?..];
            let start = section.find("<ProjectPath Value=\"")? + "<ProjectPath Value=\"".len();
            let end = start + section[start..].find('"')?;
            Some(PathBuf::from(&section[start..end]).join("User Library"))
        })
        .filter(|p| p.exists())
        .unwrap_or_else(|| config::home().join("Music/Ableton/User Library"))
}

pub fn target() -> PathBuf {
    user_library().join("Remote Scripts/LiveWeb")
}

pub fn status() -> ScriptStatus {
    let dir = target();
    if !dir.join("__init__.py").exists() {
        return ScriptStatus::NotInstalled;
    }
    let current = SCRIPT
        .files()
        .all(|f| std::fs::read(dir.join(f.path())).is_ok_and(|bytes| bytes == f.contents()));
    if current { ScriptStatus::Installed } else { ScriptStatus::Outdated }
}

pub fn install() -> Result<String, String> {
    let dir = target();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    // Drop files from older versions (and stale bytecode).
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
        let name = entry.file_name();
        if SCRIPT.get_file(&name).is_none() {
            let path = entry.path();
            let _ = if path.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };
        }
    }
    for file in SCRIPT.files() {
        let path = dir.join(file.path());
        std::fs::write(&path, file.contents()).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(format!("Remote Script installed in {} (restart Live to load it)", dir.display()))
}
