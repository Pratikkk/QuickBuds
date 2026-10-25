//! Themes, colors & styles, as the phone draws it (`ThemeActivity`, DESIGN §7.6): the style, the theme, Match system,
//! the Glass background and the accent colour. Every choice saves at once in `settings.json`.

use crate::glass::{self, style::{self, Style}};
use crate::palette::{self, Pal};
use crate::phone::{self, Trail, ROW, SEGMENT};
use crate::tokens::{self, GlassBg, Rgba};
use crate::{load_settings, save_setting, t, App};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

/// The picker order of the phone: Material, QuickBuds, Pixel art, Glass.
const ORDER: [Style; 4] = [Style::Material, Style::Classic, Style::Pixel, Style::Glass];
/// The five accent swatches of `res/values/colors.xml`.
const SWATCHES: [&str; 5] = ["#D71920", "#E8742A", "#22A06B", "#3B82F6", "#8B5CF6"];
/// The accent picker's slider block, in dp: three 40 dp tracks 45.7 dp apart, then the hex row and the recent row.
const SLIDER: f32 = 45.7;
const TRACK: f32 = 40.0;
const HEX_ROW: f32 = 152.9;
const RECENT_ROW: f32 = 190.8;

#[derive(Default)]
pub struct State {
    /// The accent picker is open under its row.
    accent_open: bool,
    /// The text of the hex field.
    hex: String,
    /// The Background sheet is open, and the choice it holds until Done.
    bg_open: bool,
    bg_pick: Option<GlassBg>,
}

// ---- the stored Glass background ----

/// `glass_bg` (the phone's `styleGlassBg`); without it the old `glass_gradient` switch means Midnight or None.
pub fn bg() -> GlassBg {
    match load_settings()["glass_bg"].as_u64() {
        Some(i) => GlassBg::ALL.get(i as usize).copied().unwrap_or(GlassBg::Midnight),
        None => if load_settings()["glass_gradient"].as_bool().unwrap_or(true) { GlassBg::Midnight } else { GlassBg::None },
    }
}

fn set_bg(b: GlassBg) {
    let i = GlassBg::ALL.iter().position(|x| *x == b).unwrap_or(0);
    save_setting("glass_bg", i.into());
    // The legacy switch is the Midnight / None pair; it stays in step for the other app.
    save_setting("glass_gradient", (b != GlassBg::None).into());
}

// ---- the Glass window ----

fn tp(c: Color32) -> Rgba { Rgba::rgb(c.r(), c.g(), c.b()) }

fn to_tokens(p: Pal) -> tokens::Palette {
    tokens::Palette { bg: tp(p.bg), card: tp(p.card), accent: tp(p.accent), text: tp(p.text), text2: tp(p.text2), outline: tp(p.outline) }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How much of a pixel at `p` is inside the rounded window.
fn coverage(p: Pos2, rect: Rect, r: f32) -> f32 {
    let q = (p - rect.center()).abs() - (rect.size() / 2.0 - Vec2::splat(r));
    let d = q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - r;
    (0.5 - d).clamp(0.0, 1.0)
}

/// Paints the Glass window: the base colour and the four radial blobs of the chosen background (`ThemeRes.glassBackground`).
/// The shader draws only the Midnight glow, so the six backgrounds are a mesh here. Glass surfaces blur what this paints.
pub fn window(ui: &Ui) {
    let rect = ui.max_rect();
    let radius = 26.0;
    let light = palette::is_light_system(ui.ctx());
    let w = tokens::window(to_tokens(palette::active(light, false)), bg());
    let aspect = rect.width() / rect.height();
    let (cols, rows) = (40usize, 26usize);
    let mut mesh = egui::epaint::Mesh::default();
    for j in 0..=rows {
        for i in 0..=cols {
            let at = Vec2::new(i as f32 / cols as f32, j as f32 / rows as f32);
            let p = Pos2::new(rect.left() + at.x * rect.width(), rect.top() + at.y * rect.height());
            let mut c = w.base;
            if let Some(blobs) = w.blobs {
                for (k, b) in blobs.iter().enumerate() {
                    let d = Vec2::new((at.x - w.centers[k].0) * aspect, at.y - w.centers[k].1).length();
                    c = tokens::blend(c, *b, w.alphas[k] * (1.0 - smoothstep(0.0, 0.62, d)));
                }
            }
            let a = coverage(p, rect, radius);
            mesh.colored_vertex(p, Color32::from_rgba_unmultiplied(c.r, c.g, c.b, (a * 255.0) as u8));
        }
    }
    for j in 0..rows {
        for i in 0..cols {
            let k = (j * (cols + 1) + i) as u32;
            let down = k + cols as u32 + 1;
            mesh.add_triangle(k, k + 1, down);
            mesh.add_triangle(k + 1, down + 1, down);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
    ui.painter().rect_stroke(rect, radius, Stroke::new(1.0, glass::white(0.26)), StrokeKind::Inside);
}

// ---- names ----

fn theme_name(tr: &[String], id: &str) -> String {
    t(tr, match id {
        palette::OLED => "theme_oled_black",
        palette::DARK => "theme_classic_dark",
        palette::WHITE => "theme_white",
        _ => "theme_custom",
    }).to_string()
}

/// The name of the active preset: a built-in theme key, or the name of a custom preset.
pub fn preset_name(tr: &[String], id: &str) -> String {
    palette::custom().into_iter().find(|p| p.0 == id).map_or_else(|| theme_name(tr, id), |p| p.1)
}

// ---- controls ----

/// A segmented control in one 43.8 dp plate. Returns the index of a click.
fn segments(ui: &mut Ui, id: &str, rect: Rect, labels: &[String], current: Option<usize>, accent: Color32) -> Option<usize> {
    glass::flat_at(ui, rect, rect.height() / 2.0, 1.0);
    let step = (rect.width() - 4.0) / labels.len() as f32;
    let mut picked = None;
    for (i, label) in labels.iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(rect.left() + 2.0 + step * i as f32, rect.top() + 2.0), Vec2::new(step, rect.height() - 4.0));
        let hit = ui.interact(r, ui.scope_id().with((id, i)), Sense::click());
        if current == Some(i) {
            // Pixel art outlines a chosen segment in the accent and writes the label in the accent (§2.2).
            if style::style() == Style::Pixel {
                ui.painter().rect_stroke(r, 4.0, Stroke::new(1.5, accent), StrokeKind::Inside);
            } else {
                ui.painter().rect_filled(r, r.height() / 2.0, accent);
            }
        } else if hit.hovered() {
            ui.painter().rect_filled(r, r.height() / 2.0, glass::white(0.07));
        }
        let colour = if current != Some(i) { glass::white(0.92) } else if style::style() == Style::Pixel { accent } else { on_accent(accent) };
        ui.painter().text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional(14.0), colour);
        if hit.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
        if hit.clicked() { picked = Some(i); }
    }
    picked
}

/// The label on an accent fill (`Palette.onAccent`).
fn on_accent(accent: Color32) -> Color32 {
    match style::style() {
        Style::Glass => tokens::glass_on_accent(tp(accent)).color32(),
        _ => style::pal().on_accent(),
    }
}

fn rgb_to_hsv(c: Color32) -> (f32, f32, f32) {
    let (r, g, b) = (c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let d = max - min;
    let h = if d == 0.0 { 0.0 } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if max == 0.0 { 0.0 } else { d / max }, max)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color32 {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match ((h / 60.0) as u32) % 6 { 0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x), 3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x) };
    let ch = |f: f32| ((f + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(ch(r), ch(g), ch(b))
}

/// A gradient track with a white thumb: the hue, saturation and brightness sliders of the accent picker.
/// Returns true when the value changed.
fn gradient(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, rect: Rect, v: &mut f32, stops: &[Color32]) -> bool {
    let hit = ui.interact(rect, ui.scope_id().with(id), Sense::click_and_drag());
    let r = rect.height() / 2.0;
    let inner = Rect::from_min_max(Pos2::new(rect.left() + r, rect.top()), Pos2::new(rect.right() - r, rect.bottom()));
    let mut changed = false;
    if let Some(p) = hit.interact_pointer_pos().filter(|_| hit.is_pointer_button_down_on()) {
        let t = ((p.x - inner.left()) / inner.width()).clamp(0.0, 1.0);
        if t != *v { *v = t; changed = true; }
    }
    let colour = |t: f32| {
        let x = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
        let i = (x.floor() as usize).min(stops.len() - 2);
        palette::blend(stops[i], stops[i + 1], x - x.floor())
    };
    // The middle of the track as a mesh, the two round ends as discs of the end colours.
    let n = 40usize;
    let mut mesh = egui::epaint::Mesh::default();
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let x = inner.left() + inner.width() * t;
        let c = colour(t);
        mesh.colored_vertex(Pos2::new(x, rect.top() + 0.5), c);
        mesh.colored_vertex(Pos2::new(x, rect.bottom() - 0.5), c);
        if i > 0 {
            let k = (i * 2) as u32;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k + 1, k);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
    ui.painter().circle_filled(Pos2::new(inner.left(), rect.center().y), r - 0.5, colour(0.0));
    ui.painter().circle_filled(Pos2::new(inner.right(), rect.center().y), r - 0.5, colour(1.0));
    let x = inner.left() + inner.width() * v.clamp(0.0, 1.0);
    ui.painter().circle_filled(Pos2::new(x, rect.center().y), 15.0, Color32::WHITE);
    ui.painter().circle_stroke(Pos2::new(x, rect.center().y), 15.0, Stroke::new(1.5, glass::white(0.55)));
    if hit.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    changed
}

/// Saves the accent on the active built-in preset, moves it to the front of Recent, and rebuilds the style.
fn commit(app: &mut App, ui: &Ui, c: Color32) {
    let id = palette::active_id(palette::is_light_system(ui.ctx()));
    if !palette::BUILT_IN.contains(&id.as_str()) { return; }
    palette::set_accent(&id, Some(c));
    let hex = palette::hex(c);
    let mut recent: Vec<String> = load_settings()["recent_colors"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
    recent.retain(|x| *x != hex);
    recent.insert(0, hex.clone());
    recent.truncate(5);
    save_setting("recent_colors", serde_json::json!(recent));
    save_setting("recent_colors_target", id.into());
    app.appearance.hex = hex;
    reload(app, ui);
}

fn reload(app: &mut App, ui: &Ui) {
    let ctx = ui.ctx().clone();
    app.reload_style(&ctx);
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    let tr = app.tr.clone();
    let light = palette::is_light_system(ui.ctx());
    let id = palette::active_id(light);
    let cur = palette::active(light, app.style == Style::Material);
    let built_in = palette::BUILT_IN.contains(&id.as_str());
    let accent = cur.accent;
    let w = phone::content(body);
    let x0 = body.left();
    let mut y = body.top();
    let card = |y: f32, h: f32| Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, h));

    // Style.
    phone::label(ui, x0, y, t(&tr, "widget_style_title"), accent);
    y += phone::SECTION;
    let labels: Vec<String> = ORDER.iter().map(|s| t(&tr, s.name_key()).to_string()).collect();
    if let Some(i) = segments(ui, "style", card(y, SEGMENT), &labels, ORDER.iter().position(|s| *s == app.style), accent) {
        save_setting("egui_style", ORDER[i].key().into());
        reload(app, ui);
    }
    y += SEGMENT;

    // Themes.
    phone::label(ui, x0, y, t(&tr, "theme_builtin"), accent);
    y += phone::SECTION;
    let labels: Vec<String> = palette::BUILT_IN.iter().map(|p| theme_name(&tr, p)).collect();
    let active = if palette::auto() { None } else { palette::BUILT_IN.iter().position(|p| *p == id) };
    if let Some(i) = segments(ui, "theme", card(y, SEGMENT), &labels, active, accent) {
        palette::set_active(palette::BUILT_IN[i]);
        reload(app, ui);
    }
    y += SEGMENT + phone::GAP;

    // Match system, and the Glass background under it.
    let rows = 1 + if app.style == Style::Glass { 1 } else { 0 };
    glass::flat_at(ui, card(y, ROW * rows as f32 + 8.0 * (rows - 1) as f32), phone::RADIUS, 1.0);
    let auto = palette::auto();
    let dark = load_settings()["palette_auto_dark"].as_str().unwrap_or(palette::OLED).to_string();
    let sub = t(&tr, "theme_auto_sub").replace("%1$s", &theme_name(&tr, palette::WHITE)).replace("%2$s", &preset_name(&tr, &dark));
    if phone::row_at(ui, "auto", card(y, ROW), phone::Row::new(t(&tr, "theme_auto"), &sub, Trail::Switch(auto), accent), true).1 {
        palette::set_auto(!auto);
        reload(app, ui);
    }
    y += ROW;
    if app.style == Style::Glass {
        y += 8.0;
        let name = t(&tr, bg().name_key()).to_string();
        if phone::row_at(ui, "bg", card(y, ROW), phone::Row::new(t(&tr, "theme_glass_gradient"), &name, Trail::Chevron, accent), true).0 {
            app.appearance.bg_open = true;
            app.appearance.bg_pick = Some(bg());
        }
        y += ROW;
    }
    y += phone::GAP;

    // The accent row, and the picker under it.
    let hex = palette::hex(accent);
    let sub = t(&tr, "theme_accent_sub").replace("%1$s", &preset_name(&tr, &id)).replace("%2$s", &hex);
    let trail = if app.appearance.accent_open { Trail::ChevronOpen } else { Trail::Chevron };
    let row = phone::Row::new(t(&tr, "theme_accent"), &sub, trail, accent).swatch(accent);
    let picker = if app.appearance.accent_open { RECENT_ROW + 55.6 } else { 0.0 };
    glass::flat_at(ui, card(y, ROW + picker), phone::RADIUS, 1.0);
    if phone::row_at(ui, "accent", card(y, ROW), row, built_in).0 {
        app.appearance.accent_open = !app.appearance.accent_open;
        app.appearance.hex = hex.clone();
    }
    y += ROW;
    if app.appearance.accent_open {
        let base = body.left() + phone::MARGIN;
        let (mut hue, mut sat, mut val) = rgb_to_hsv(accent);
        let mut pick: Option<Color32> = None;
        let track = Vec2::new(w - 2.0 * phone::MARGIN, TRACK);
        let hue_stops: Vec<Color32> = (0..=6).map(|i| hsv_to_rgb(i as f32 * 60.0, 1.0, 1.0)).collect();
        // The track holds a 0..1 position; the hue is 0..360.
        let mut hue_t = hue / 360.0;
        if gradient(ui, "hue", Rect::from_min_size(Pos2::new(base, y + 3.8), track), &mut hue_t, &hue_stops) {
            hue = hue_t * 360.0;
            pick = Some(hsv_to_rgb(hue, sat, val));
        }
        let sat_stops = [hsv_to_rgb(hue, 0.0, val), hsv_to_rgb(hue, 1.0, val)];
        if gradient(ui, "sat", Rect::from_min_size(Pos2::new(base, y + 3.8 + SLIDER), track), &mut sat, &sat_stops) { pick = Some(hsv_to_rgb(hue, sat, val)); }
        let val_stops = [Color32::BLACK, hsv_to_rgb(hue, sat, 1.0)];
        if gradient(ui, "val", Rect::from_min_size(Pos2::new(base, y + 3.8 + 2.0 * SLIDER), track), &mut val, &val_stops) { pick = Some(hsv_to_rgb(hue, sat, val)); }

        // The hex field, then the five swatches of the phone against the right edge.
        let hex_rect = Rect::from_min_size(Pos2::new(base, y + HEX_ROW), Vec2::new(112.0, 43.8));
        let field = glass::text_field_at(ui, hex_rect, &mut app.appearance.hex, 7);
        if field.lost_focus() || (field.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
            if let Some(c) = palette::parse_hex(&app.appearance.hex) { pick = Some(c); }
            app.appearance.hex = hex.clone();
        }
        let size = 29.7;
        let gap = 8.0;
        let left = base + phone::MARGIN + (w - 2.0 * phone::MARGIN) - (5.0 * size + 4.0 * gap);
        for (n, s) in SWATCHES.iter().enumerate() {
            let c = palette::parse_hex(s).unwrap();
            let centre = Pos2::new(left + (size + gap) * n as f32 + size / 2.0, hex_rect.center().y);
            let hit = ui.interact(Rect::from_center_size(centre, Vec2::splat(size + 10.0)), ui.scope_id().with(("swatch", n)), Sense::click());
            ui.painter().circle_filled(centre, size / 2.0 + if hit.hovered() { 1.5 } else { 0.0 }, c);
            if accent == c { ui.painter().circle_stroke(centre, size / 2.0 + 3.0, Stroke::new(2.0, glass::white(1.0))); }
            if hit.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
            if hit.clicked() { pick = Some(c); }
        }

        // Recent, the last five accents, newest first.
        let recent_y = y + RECENT_ROW;
        glass::text(ui, Pos2::new(base, recent_y + 20.0), Align2::LEFT_CENTER, t(&tr, "preset_recent"), 13.0, 0.74);
        let recent: Vec<String> = load_settings()["recent_colors"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        for (n, s) in recent.iter().take(5).enumerate() {
            if let Some(c) = palette::parse_hex(s) {
                let centre = Pos2::new(base + w - 2.0 * phone::MARGIN - size / 2.0 - n as f32 * (size + gap), recent_y + 20.0);
                let hit = ui.interact(Rect::from_center_size(centre, Vec2::splat(size + 10.0)), ui.scope_id().with(("recent", n)), Sense::click());
                ui.painter().circle_filled(centre, size / 2.0, c);
                if hit.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
                if hit.clicked() { pick = Some(c); }
            }
        }
        if let Some(c) = pick { commit(app, ui, c); }
        y += RECENT_ROW + 55.6;
    }
    y + phone::GAP
}

/// The six backgrounds, in a sheet with Done and Cancel, as the phone shows them.
pub fn sheet(app: &mut App, ui: &mut Ui) {
    if !app.appearance.bg_open { return; }
    let tr = app.tr.clone();
    let mut options: Vec<String> = Vec::new();
    for b in GlassBg::ALL { options.push(t(&tr, b.name_key()).to_string()); }
    let mut pick = app.appearance.bg_pick;
    let labels = options.clone();
    let mut choice = None;
    let open = glass::dialog(ui.ctx(), "glassbg", t(&tr, "theme_glass_gradient"), Vec2::new(380.0, 430.0), |ui| {
        let r = ui.max_rect();
        for (i, label) in labels.iter().enumerate() {
            let row = Rect::from_min_size(Pos2::new(r.left() - 12.0, r.top() + 52.0 * i as f32 - 8.0), Vec2::new(r.width() + 24.0, 52.0));
            let selected = pick == Some(GlassBg::ALL[i]);
            if glass::list_item_at(ui, ("gbg", i), row, label, selected, false).clicked() { pick = Some(GlassBg::ALL[i]); }
        }
        choice = glass::dialog_buttons(ui, t(&tr, "dialog_cancel"), t(&tr, "gesture_done"));
    });
    app.appearance.bg_pick = pick;
    match choice {
        Some(true) => {
            if let Some(b) = pick { set_bg(b); }
            app.appearance.bg_open = false;
        }
        Some(false) => app.appearance.bg_open = false,
        None if !open => app.appearance.bg_open = false,
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_round_trip() {
        for hex in ["#D71920", "#22A06B", "#3B82F6", "#000000", "#FFFFFF", "#C2C6D6", "#FF8CF7"] {
            let c = palette::parse_hex(hex).unwrap();
            let (h, s, v) = rgb_to_hsv(c);
            assert_eq!(hsv_to_rgb(h, s, v), c, "{hex}");
        }
    }

    /// Every accent swatch of the phone is a readable 6-digit colour.
    #[test]
    fn swatches_parse() {
        for s in SWATCHES { assert!(palette::parse_hex(s).is_some(), "{s}"); }
    }

    /// The window is fully inside its own centre, and outside its corner.
    #[test]
    fn window_coverage() {
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(400.0, 200.0));
        assert_eq!(coverage(rect.center(), rect, 26.0), 1.0);
        assert_eq!(coverage(rect.min, rect, 26.0), 0.0);
    }
}
