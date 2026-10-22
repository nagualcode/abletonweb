//! Flat, dark look inspired by Ableton Live: grey panels, square-ish
//! corners, small type and the orange/yellow accent for active state.

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals};

pub const BG: Color32 = Color32::from_rgb(0x1e, 0x1e, 0x1e);
pub const PANEL: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2b);
pub const BUTTON: Color32 = Color32::from_rgb(0x3c, 0x3c, 0x3c);
pub const BUTTON_HOVER: Color32 = Color32::from_rgb(0x4a, 0x4a, 0x4a);
pub const LINE: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
pub const TEXT: Color32 = Color32::from_rgb(0xd6, 0xd6, 0xd6);
pub const MUTED: Color32 = Color32::from_rgb(0x8c, 0x8c, 0x8c);
pub const ACCENT: Color32 = Color32::from_rgb(0xff, 0xb5, 0x32);
pub const GREEN: Color32 = Color32::from_rgb(0x3d, 0xdc, 0x84);
pub const RED: Color32 = Color32::from_rgb(0xff, 0x3b, 0x30);

const RADIUS: CornerRadius = CornerRadius::same(2);

pub fn apply(ctx: &egui::Context) {
    let mut v = Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = LINE;
    v.faint_bg_color = PANEL;
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = ACCENT.linear_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;
    v.window_corner_radius = RADIUS;
    v.menu_corner_radius = RADIUS;

    let w = &mut v.widgets;
    for (state, fill) in [
        (&mut w.inactive, BUTTON),
        (&mut w.hovered, BUTTON_HOVER),
        (&mut w.active, ACCENT),
        (&mut w.open, BUTTON),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.corner_radius = RADIUS;
        state.bg_stroke = Stroke::NONE;
        state.expansion = 0.0;
    }
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    w.noninteractive.corner_radius = RADIUS;
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    w.active.fg_stroke = Stroke::new(1.0, BG);
    // Always dark, whatever the macOS appearance is.
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, v);

    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, FontId::new(10.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
            (TextStyle::Heading, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
        ]
        .into();
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        style.spacing.interact_size.y = 24.0;
    });
}

/// A section card with a small uppercase caption, like Live's browser headers.
pub fn section<R>(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(title.to_uppercase()).small().color(MUTED).strong());
    egui::Frame::new()
        .fill(PANEL)
        .corner_radius(RADIUS)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Coloured status dot followed by text.
pub fn dot_label(ui: &mut egui::Ui, color: Color32, text: impl Into<egui::WidgetText>) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, color);
        ui.label(text);
    });
}

/// A toggle button that lights up in the accent colour, like Live's buttons.
pub fn toggle(ui: &mut egui::Ui, on: bool, text: &str) -> egui::Response {
    let fill = if on { ACCENT } else { BUTTON };
    let color = if on { BG } else { TEXT };
    ui.add(egui::Button::new(egui::RichText::new(text).color(color)).fill(fill))
}
