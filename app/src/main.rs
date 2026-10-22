//! Live Remote: control Ableton Live from a phone or tablet browser.
//!
//! One binary: without arguments it opens the control panel; `serve` runs
//! the bridge (what the LaunchAgent starts); the other commands let you
//! install and manage everything from a terminal, without the window.

mod actions;
mod bridge;
mod config;
mod diff;
mod gui;
mod remote_script;
mod service;
mod status;
mod theme;

const HELP: &str = "\
Live Remote — control Ableton Live from a browser

Usage: live-remote [command]

  (no command)          open the control panel
  install               install the Remote Script and the bridge service (starts at login)
  uninstall             remove the bridge service
  restart               restart the bridge service
  status                show service, bridge and Live status
  url                   print the address (and a QR code) to open on your phone
  install-script        install/update only the LiveWeb Remote Script
  serve [--port N]      run the bridge in the foreground

Settings: ~/.config/abletonweb/config.json (port, requireToken)";

fn main() {
    // Finder may pass a -psn_* argument to apps on older macOS versions.
    let args: Vec<String> = std::env::args().skip(1).filter(|a| !a.starts_with("-psn")).collect();
    let result = match args.first().map(String::as_str) {
        None => gui::run().map(|_| String::new()).map_err(|e| e.to_string()),
        Some("serve") => serve(&args[1..]),
        Some("install") => remote_script::install().and_then(|s| Ok(format!("{s}\n{}", service::install()?))),
        Some("uninstall") => service::uninstall(),
        Some("restart") => service::restart(),
        Some("install-script") => remote_script::install(),
        Some("status") => Ok(status_text()),
        Some("url") => Ok(url_text()),
        Some("help" | "-h" | "--help") => Ok(HELP.to_string()),
        Some(other) => Err(format!("unknown command {other:?}\n\n{HELP}")),
    };
    match result {
        Ok(out) if !out.is_empty() => println!("{out}"),
        Ok(_) => {}
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

fn serve(args: &[String]) -> Result<String, String> {
    let mut config = config::load_or_init();
    if let Some(i) = args.iter().position(|a| a == "--port") {
        config.port = args.get(i + 1).and_then(|p| p.parse().ok()).ok_or("--port needs a number")?;
    }
    if let Ok(port) = std::env::var("PORT") {
        config.port = port.parse().map_err(|_| "PORT must be a number")?;
    }
    if let Ok(port) = std::env::var("LIVE_PORT") {
        config.live_port = port.parse().map_err(|_| "LIVE_PORT must be a number")?;
    }
    bridge::serve(config).map(|_| String::new())
}

fn status_text() -> String {
    let config = config::load();
    let s = status::check(config.port);
    let service = match s.service {
        service::Service::Running(pid) => format!("running (pid {pid})"),
        service::Service::Stopped => "installed, not running".into(),
        service::Service::NotInstalled => "not installed".into(),
        service::Service::Unknown => "unknown".into(),
    };
    let live = match &s.health {
        Some(h) if h.live => format!("connected (Live {}), {} device(s)", h.version, h.clients),
        Some(_) => "waiting for Live (is LiveWeb selected as Control Surface?)".into(),
        None => format!("bridge not answering on port {}", config.port),
    };
    let script = match s.remote_script {
        remote_script::ScriptStatus::Installed => "installed",
        remote_script::ScriptStatus::Outdated => "outdated (run: live-remote install-script)",
        remote_script::ScriptStatus::NotInstalled => "not installed",
    };
    format!(
        "service:       {service}\nlive:          {live}\nremote script: {script} ({})\nlog:           {}",
        remote_script::target().display(),
        service::log_path().display()
    )
}

fn url_text() -> String {
    let config = config::load_or_init();
    let query = match (&config.require_token, &config.token) {
        (true, Some(t)) => format!("?t={t}"),
        _ => String::new(),
    };
    let urls: Vec<String> = [gui::local_hostname(), gui::lan_ip()]
        .into_iter()
        .flatten()
        .map(|host| format!("http://{host}:{}/{query}", config.port))
        .collect();
    let mut out = urls.join("\n");
    if let Some(url) = urls.first() {
        if let Ok(code) = qrcode::QrCode::new(url.as_bytes()) {
            out.push_str("\n\n");
            out.push_str(&qr_terminal(&code));
        }
    }
    out
}

/// Renders a QR code with half-block characters (two rows per line).
fn qr_terminal(code: &qrcode::QrCode) -> String {
    let width = code.width();
    let colors = code.to_colors();
    let dark = |x: i64, y: i64| {
        x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < width
            && colors[y as usize * width + x as usize] == qrcode::Color::Dark
    };
    let mut out = String::new();
    let (lo, hi) = (-2i64, width as i64 + 2);
    let mut y = lo;
    while y < hi {
        for x in lo..hi {
            out.push(match (dark(x, y), dark(x, y + 1)) {
                (true, true) => ' ',
                (true, false) => '▄',
                (false, true) => '▀',
                (false, false) => '█',
            });
        }
        out.push('\n');
        y += 2;
    }
    out
}
