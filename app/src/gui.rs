//! The control panel window: installs, monitors and removes the bridge
//! service and shows how to connect a phone. The bridge itself runs without
//! it (as a LaunchAgent).

use crate::actions::{self, Runner};
use crate::config;
use crate::remote_script::{self, ScriptStatus};
use crate::service::{self, Service};
use crate::status::{self, Status};
use crate::theme::{self, ACCENT, GREEN, MUTED, RED};
use eframe::egui::{self, Color32, RichText};
use std::sync::{Arc, Mutex};

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Live Remote")
            .with_inner_size([400.0, 860.0])
            .with_min_inner_size([360.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native("Live Remote", options, Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx)))))
}

#[derive(Clone, Copy, PartialEq)]
enum UrlKind {
    Hostname,
    Ip,
}

struct App {
    status: Arc<Mutex<Status>>,
    runner: Runner,
    config: config::Config,
    /// Edited copy of the config, applied with the Apply button.
    draft: config::Config,
    port_text: String,
    url_kind: UrlKind,
    hostname: Option<String>,
    confirm_uninstall: bool,
    /// Dev aid: LIVE_REMOTE_SCREENSHOT=file.png saves the window and quits.
    screenshot: Option<(String, std::time::Instant)>,
}

impl App {
    fn new(ctx: &egui::Context) -> Self {
        theme::apply(ctx);
        // The app may have been moved or updated since the service was installed.
        service::repair_if_moved();
        let config = config::load_or_init();
        App {
            status: status::spawn_poller(ctx.clone()),
            runner: Runner::default(),
            port_text: config.port.to_string(),
            draft: config.clone(),
            config,
            url_kind: UrlKind::Hostname,
            hostname: local_hostname(),
            confirm_uninstall: false,
            screenshot: std::env::var("LIVE_REMOTE_SCREENSHOT").ok().map(|p| (p, std::time::Instant::now())),
        }
    }

    fn url(&self) -> Option<String> {
        let host = match self.url_kind {
            UrlKind::Hostname => self.hostname.clone()?,
            UrlKind::Ip => lan_ip()?,
        };
        Some(self.url_for(&host))
    }

    fn url_for(&self, host: &str) -> String {
        let mut url = format!("http://{host}:{}/", self.config.port);
        if self.config.require_token {
            if let Some(token) = &self.config.token {
                url.push_str(&format!("?t={token}"));
            }
        }
        url
    }

    fn apply_config(&mut self, ctx: &egui::Context, service: &Service) {
        // Keep a token the bridge may have written since we loaded the file.
        let current = config::load();
        self.draft.token = current.token.or(self.draft.token.take());
        if let Err(e) = config::save(&self.draft) {
            eprintln!("cannot save config: {e}");
            return;
        }
        self.config = self.draft.clone();
        if matches!(service, Service::Running(_) | Service::Stopped) {
            self.runner.run("Restarting", ctx, service::restart);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let status = self.status.lock().unwrap().clone();
        let ctx = ui.ctx().clone();
        self.maybe_screenshot(&ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).inner_margin(egui::Margin::same(14)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.header(ui, &status);
                    self.status_section(ui, &status);
                    self.connect_section(ui, &status);
                    self.config_section(ui, &ctx, &status);
                    self.service_section(ui, &ctx, &status);
                });
            });
    }
}

impl App {
    fn header(&self, ui: &mut egui::Ui, status: &Status) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("LIVE REMOTE").heading().strong().color(ACCENT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (color, text) = match &status.health {
                    Some(h) if h.live => (GREEN, "Ready"),
                    Some(_) => (ACCENT, "Waiting for Live"),
                    None => (RED, "Bridge stopped"),
                };
                ui.label(RichText::new(text).small().color(MUTED));
                let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 5.0, color);
            });
        });
    }

    fn status_section(&self, ui: &mut egui::Ui, status: &Status) {
        theme::section(ui, "Status", |ui| {
            egui::Grid::new("status").num_columns(2).spacing([16.0, 8.0]).show(ui, |ui| {
                ui.label(RichText::new("Service").color(MUTED));
                match &status.service {
                    Service::Running(pid) => theme::dot_label(ui, GREEN, format!("Running (pid {pid})")),
                    Service::Stopped => theme::dot_label(ui, ACCENT, "Installed, stopped"),
                    Service::NotInstalled => theme::dot_label(ui, MUTED, "Not installed"),
                    Service::Unknown => theme::dot_label(ui, MUTED, "…"),
                }
                ui.end_row();

                ui.label(RichText::new("Bridge").color(MUTED));
                match &status.health {
                    Some(_) => theme::dot_label(ui, GREEN, format!("Port {}", self.config.port)),
                    None => theme::dot_label(ui, RED, format!("Not answering on port {}", self.config.port)),
                }
                ui.end_row();

                ui.label(RichText::new("Ableton Live").color(MUTED));
                match &status.health {
                    Some(h) if h.live => theme::dot_label(ui, GREEN, format!("Connected · {}", h.version)),
                    Some(_) => theme::dot_label(ui, ACCENT, "Waiting (Live closed or LiveWeb not selected)"),
                    None => theme::dot_label(ui, MUTED, "—"),
                }
                ui.end_row();

                ui.label(RichText::new("Remote Script").color(MUTED));
                match status.remote_script {
                    ScriptStatus::Installed => theme::dot_label(ui, GREEN, "LiveWeb installed"),
                    ScriptStatus::Outdated => theme::dot_label(ui, ACCENT, "Outdated"),
                    ScriptStatus::NotInstalled => theme::dot_label(ui, RED, "Not installed"),
                }
                ui.end_row();

                ui.label(RichText::new("Devices").color(MUTED));
                let clients = status.health.as_ref().map_or(0, |h| h.clients);
                ui.label(match clients {
                    0 => "None connected".to_string(),
                    1 => "1 connected".to_string(),
                    n => format!("{n} connected"),
                });
                ui.end_row();
            });
        });
    }

    fn connect_section(&mut self, ui: &mut egui::Ui, status: &Status) {
        theme::section(ui, "Connect", |ui| {
            if status.health.is_none() {
                ui.label(RichText::new("Install or start the service to connect a device.").color(MUTED));
                return;
            }
            ui.horizontal(|ui| {
                if theme::toggle(ui, self.url_kind == UrlKind::Hostname, "Name .local").clicked() {
                    self.url_kind = UrlKind::Hostname;
                }
                if theme::toggle(ui, self.url_kind == UrlKind::Ip, "IP").clicked() {
                    self.url_kind = UrlKind::Ip;
                }
            });
            let Some(url) = self.url() else {
                ui.label(RichText::new("No network address available.").color(RED));
                return;
            };
            ui.add_space(6.0);
            ui.vertical_centered(|ui| qr(ui, &url, 180.0));
            ui.add_space(6.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(&url).monospace());
                ui.horizontal(|ui| {
                    // Center the two buttons.
                    let width = 150.0;
                    ui.add_space((ui.available_width() - width).max(0.0) / 2.0);
                    if ui.button("Copy").clicked() {
                        ui.ctx().copy_text(url.clone());
                    }
                    if ui.button("Open").clicked() {
                        actions::open_url(&self.url_for("localhost"));
                    }
                });
            });
            ui.label(
                RichText::new("Point your phone camera at the code. Same Wi-Fi as this Mac.")
                    .small()
                    .color(MUTED),
            );
        });
    }

    fn config_section(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, status: &Status) {
        theme::section(ui, "Settings", |ui| {
            egui::Grid::new("config").num_columns(2).spacing([16.0, 8.0]).show(ui, |ui| {
                ui.label(RichText::new("Port").color(MUTED));
                let edit = ui.add(egui::TextEdit::singleline(&mut self.port_text).desired_width(70.0));
                if edit.changed() {
                    self.port_text.retain(|c| c.is_ascii_digit());
                }
                ui.end_row();

                ui.label(RichText::new("Token").color(MUTED));
                let label = if self.draft.require_token { "Required" } else { "Off" };
                if theme::toggle(ui, self.draft.require_token, label).clicked() {
                    self.draft.require_token = !self.draft.require_token;
                }
                ui.end_row();
            });

            let port = self.port_text.parse::<u16>().ok().filter(|p| *p >= 1024);
            if let Some(p) = port {
                self.draft.port = p;
            }
            let changed = self.draft.port != self.config.port || self.draft.require_token != self.config.require_token;
            ui.horizontal(|ui| {
                let apply = ui.add_enabled(changed && port.is_some(), egui::Button::new("Apply"));
                if apply.clicked() {
                    self.apply_config(ctx, &status.service);
                }
                if port.is_none() {
                    ui.label(RichText::new("Port between 1024 and 65535").small().color(RED));
                } else if changed {
                    ui.label(RichText::new("The service will be restarted.").small().color(MUTED));
                }
            });
        });
    }

    fn service_section(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, status: &Status) {
        theme::section(ui, "Service", |ui| {
            let busy = self.runner.busy();
            ui.add_enabled_ui(busy.is_none(), |ui| {
                ui.horizontal_wrapped(|ui| {
                    match status.service {
                        Service::NotInstalled | Service::Unknown => {
                            let install = egui::Button::new(RichText::new("Install").color(theme::BG)).fill(ACCENT);
                            if ui.add(install).on_hover_text("Installs the bridge service and the Remote Script").clicked() {
                                self.runner.run("Installing", ctx, install_all);
                            }
                        }
                        Service::Running(_) | Service::Stopped => {
                            if ui.button("Restart").clicked() {
                                self.runner.run("Restarting", ctx, service::restart);
                            }
                            if self.confirm_uninstall {
                                let confirm = egui::Button::new(RichText::new("Confirm uninstall").color(Color32::WHITE)).fill(RED);
                                if ui.add(confirm).clicked() {
                                    self.confirm_uninstall = false;
                                    self.runner.run("Uninstalling", ctx, service::uninstall);
                                }
                                if ui.button("Cancel").clicked() {
                                    self.confirm_uninstall = false;
                                }
                            } else if ui.button("Uninstall").clicked() {
                                self.confirm_uninstall = true;
                            }
                        }
                    }
                    if status.remote_script != ScriptStatus::Installed {
                        let label = if status.remote_script == ScriptStatus::Outdated {
                            "Update Remote Script"
                        } else {
                            "Install Remote Script"
                        };
                        if ui.button(label).clicked() {
                            self.runner.run("Remote Script", ctx, remote_script::install);
                        }
                    }
                });
            });
            ui.add_space(4.0);
            if let Some(label) = busy {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(format!("{label}…")).color(MUTED));
                });
            } else if let Some(last) = self.runner.last() {
                let color = if last.ok { MUTED } else { RED };
                ui.label(RichText::new(format!("{}: {}", last.label, if last.ok { "done" } else { "failed" })).small().color(color));
                if !last.output.is_empty() {
                    egui::ScrollArea::vertical().max_height(90.0).id_salt("output").show(ui, |ui| {
                        ui.label(RichText::new(&last.output).monospace().small().color(MUTED));
                    });
                }
            }
            if status.remote_script != ScriptStatus::NotInstalled && status.health.as_ref().is_some_and(|h| !h.live) {
                ui.label(
                    RichText::new("To connect: restart Live and choose LiveWeb as Control Surface in Settings → Link, Tempo & MIDI (Input/Output: None).")
                        .small()
                        .color(MUTED),
                );
            }
        });
    }
}

impl App {
    fn maybe_screenshot(&mut self, ctx: &egui::Context) {
        let Some((path, started)) = &self.screenshot else { return };
        // Give the poller time to fill in the status first.
        if started.elapsed() < std::time::Duration::from_millis(2500) {
            ctx.request_repaint();
            return;
        }
        let path = path.clone();
        self.screenshot = None;
        ctx.request_screenshot(move |image| {
            let file = std::fs::File::create(&path).expect("create screenshot file");
            let mut encoder = png::Encoder::new(file, image.width() as u32, image.height() as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("png header");
            writer.write_image_data(image.as_raw()).expect("png data");
            drop(writer);
            std::process::exit(0);
        });
    }
}

fn install_all() -> Result<String, String> {
    let script = remote_script::install()?;
    let svc = service::install()?;
    Ok(format!("{svc}\n{script}"))
}

/// Paints a QR code as dark modules on a light square (what cameras expect).
fn qr(ui: &mut egui::Ui, text: &str, size: f32) {
    let Ok(code) = qrcode::QrCode::new(text.as_bytes()) else { return };
    let width = code.width();
    let colors = code.to_colors();
    let quiet = 2;
    let cells = width + quiet * 2;
    let cell = (size / cells as f32).floor();
    let side = cell * cells as f32;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, Color32::from_rgb(0xf2, 0xf2, 0xf2));
    for (i, c) in colors.iter().enumerate() {
        if *c == qrcode::Color::Dark {
            let x = rect.min.x + (i % width + quiet) as f32 * cell;
            let y = rect.min.y + (i / width + quiet) as f32 * cell;
            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell, cell)), 0.0, theme::BG);
        }
    }
}

pub fn local_hostname() -> Option<String> {
    let out = std::process::Command::new("scutil").args(["--get", "LocalHostName"]).output().ok()?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!name.is_empty()).then(|| format!("{name}.local"))
}

/// First private IPv4 address, preferring Wi-Fi/Ethernet (en*).
pub fn lan_ip() -> Option<String> {
    let mut addrs: Vec<_> = if_addrs::get_if_addrs()
        .ok()?
        .into_iter()
        .filter(|a| !a.is_loopback() && a.ip().is_ipv4())
        .collect();
    addrs.sort_by_key(|a| !a.name.starts_with("en"));
    addrs.first().map(|a| a.ip().to_string())
}
