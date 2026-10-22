//! The bridge as a macOS LaunchAgent running `<this executable> serve`:
//! starts at login and is restarted by launchd if it dies.

use crate::config;
use std::path::PathBuf;
use std::process::Command;

pub const LABEL: &str = "com.abletonweb.bridge";

#[derive(Clone, Debug, PartialEq)]
pub enum Service {
    Unknown,
    NotInstalled,
    Stopped,
    Running(u32),
}

pub fn plist_path() -> PathBuf {
    config::home().join(format!("Library/LaunchAgents/{LABEL}.plist"))
}

pub fn log_path() -> PathBuf {
    config::home().join("Library/Logs/abletonweb/bridge.log")
}

fn domain() -> String {
    let uid = Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "501".into());
    format!("gui/{uid}")
}

/// The executable launchd should run. Refuses macOS "App Translocation"
/// paths (an app opened straight from Downloads runs from a random,
/// temporary location that disappears).
fn executable() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.canonicalize().unwrap_or(exe);
    if exe.to_string_lossy().contains("/AppTranslocation/") {
        return Err("Move Live Remote to the Applications folder and open it again before installing.".into());
    }
    Ok(exe)
}

pub fn status() -> Service {
    let Ok(out) = Command::new("launchctl").arg("print").arg(format!("{}/{LABEL}", domain())).output() else {
        return Service::Unknown;
    };
    if !out.status.success() {
        return Service::NotInstalled;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.trim().strip_prefix("pid = ").and_then(|p| p.trim().parse().ok()))
        .map(Service::Running)
        .unwrap_or(Service::Stopped)
}

pub fn install() -> Result<String, String> {
    let exe = executable()?;
    let log = log_path();
    std::fs::create_dir_all(log.parent().unwrap()).map_err(|e| e.to_string())?;
    let plist = plist_path();
    std::fs::create_dir_all(plist.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&plist, plist_xml(&exe, &log)).map_err(|e| e.to_string())?;
    unload();
    launchctl(&["bootstrap", &domain(), &plist.to_string_lossy()])?;
    Ok(format!("Service installed ({})", exe.display()))
}

pub fn uninstall() -> Result<String, String> {
    unload();
    let plist = plist_path();
    if plist.exists() {
        std::fs::remove_file(&plist).map_err(|e| e.to_string())?;
    }
    Ok("Service removed".into())
}

pub fn restart() -> Result<String, String> {
    launchctl(&["kickstart", "-k", &format!("{}/{LABEL}", domain())])?;
    Ok("Service restarted".into())
}

/// If the service points at another copy of the app (moved or updated),
/// re-installs it for this one. Returns true when it did.
pub fn repair_if_moved() -> bool {
    let Ok(plist) = std::fs::read_to_string(plist_path()) else { return false };
    let Ok(exe) = executable() else { return false };
    if plist.contains(&format!("<string>{}</string>", xml_escape(&exe.to_string_lossy()))) {
        return false;
    }
    install().is_ok()
}

/// Stops and unloads the service, waiting until launchd has let it go:
/// `bootout` returns early, and bootstrapping again too soon fails with
/// "Input/output error".
fn unload() {
    let target = format!("{}/{LABEL}", domain());
    if launchctl(&["bootout", &target]).is_err() {
        return;
    }
    for _ in 0..50 {
        if status() == Service::NotInstalled {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn launchctl(args: &[&str]) -> Result<(), String> {
    let out = Command::new("launchctl").args(args).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("launchctl {}: {}", args[0], String::from_utf8_lossy(&out.stderr).trim()))
    }
}

fn plist_xml(exe: &std::path::Path, log: &std::path::Path) -> String {
    let exe = xml_escape(&exe.to_string_lossy());
    let log = xml_escape(&log.to_string_lossy());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
    <string>serve</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>ThrottleInterval</key>
  <integer>5</integer>
  <key>ProcessType</key>
  <string>Interactive</string>
  <key>StandardOutPath</key>
  <string>{log}</string>
  <key>StandardErrorPath</key>
  <string>{log}</string>
</dict>
</plist>
"#
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
