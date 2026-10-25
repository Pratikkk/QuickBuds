//! The phone's list row and section label (DESIGN §4.3, §7.4 and §7.12), shared by Settings and Earbud settings.
//! A row is its own 16 dp card, 59.8 dp high, or taller when its subtitle wraps.

use crate::glass;
use crate::icons;
use crate::phone;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Ui, Vec2};

/// A split list row.
pub const RADIUS: f32 = 16.0;
/// The left edge of the text: a 16 dp margin, a 24 dp icon and a 13.7 dp gap.
pub const TEXT_LEFT: f32 = 53.7;
/// The width kept for a chevron or a switch at the end of a row.
pub const TRAIL: f32 = 68.0;

/// What a row shows on its right.
#[derive(Clone, Copy)]
pub enum Trail<'a> {
    None,
    Chevron,
    Switch(bool),
    Value(&'a str),
}

/// The height of a row: 56 dp with no subtitle, else the title line and the wrapped subtitle with 8 dp pads.
pub fn row_height(ui: &Ui, sub: &str, w: f32) -> f32 {
    if sub.is_empty() { return 56.0; }
    let text_w = (w - TEXT_LEFT - TRAIL).max(40.0);
    (8.0 + 23.2 + phone::wrap_height(ui, sub, 13.0, text_w) + 8.0).max(59.8)
}

/// Cuts a line with an ellipsis before the trailing control. The width is measured, not estimated.
pub fn fit(ui: &Ui, s: &str, size: f32, width: f32) -> String {
    let font = FontId::proportional(size);
    let measure = |text: &str| ui.painter().layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE).size().x;
    if measure(s) <= width { return s.to_string(); }
    let mut cut = s.to_string();
    while !cut.is_empty() {
        cut.pop();
        let shown = format!("{}…", cut.trim_end());
        if measure(&shown) <= width { return shown; }
    }
    String::new()
}

/// A section label: 14 sp, secondary in Glass, else the accent, in a 41.9 dp box.
pub fn section(ui: &Ui, x: f32, y: f32, text: &str, accent: Color32) {
    let colour = if glass::style::glass() { glass::white(0.70) } else { accent };
    ui.painter().text(Pos2::new(x + 4.0, y + phone::SECTION / 2.0), Align2::LEFT_CENTER, text, FontId::proportional(14.0), colour);
}

/// A row of 59.8 dp or more: an icon, a 16 sp title, a wrapped 13 sp subtitle, and a chevron, switch or value.
/// Returns (row clicked, switch toggled). A tap anywhere on a switch row toggles it, as on the phone.
pub fn row_at(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug + Copy,
    rect: Rect,
    icon: Option<&'static str>,
    title: &str,
    sub: &str,
    trail: Trail<'_>,
    enabled: bool,
) -> (bool, bool) {
    let r = ui.interact(rect, ui.scope_id().with(id), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(r.id.with("h"), r.hovered() && enabled, 0.12);
    if hover > 0.0 { ui.painter().rect_filled(rect.shrink2(Vec2::new(8.0, 4.0)), 20.0, glass::white(0.07 * hover)); }
    let a = if enabled { 1.0 } else { 0.5 };
    if let Some(svg) = icon {
        glass::icon_at(ui, svg, Rect::from_center_size(Pos2::new(rect.left() + phone::MARGIN + 12.0, rect.center().y), Vec2::splat(24.0)), glass::white(0.78 * a));
    }
    let x = rect.left() + TEXT_LEFT;
    let text_w = (rect.right() - TRAIL - x).max(40.0);
    let colour = glass::white(a);
    if sub.is_empty() {
        glass::text(ui, Pos2::new(x, rect.center().y), Align2::LEFT_CENTER, &fit(ui, title, 16.0, text_w), 16.0, a);
    } else {
        glass::text(ui, Pos2::new(x, rect.top() + 19.6), Align2::LEFT_CENTER, &fit(ui, title, 16.0, text_w), 16.0, a);
        let galley = ui.painter().layout(sub.to_string(), FontId::proportional(13.0), glass::white(0.74 * a), text_w);
        ui.painter().galley(Pos2::new(x, rect.top() + 31.2), galley, glass::white(0.74 * a));
    }
    let mut toggled = false;
    match trail {
        Trail::None => {}
        Trail::Chevron => phone::chevron(ui, Pos2::new(rect.right() - phone::MARGIN - 12.0, rect.center().y), false, glass::white(0.72 * a)),
        Trail::Switch(on) => toggled = glass::switch_at(ui, (id, "sw"), Pos2::new(rect.right() - phone::MARGIN - 52.0, rect.center().y - 16.0), on, enabled),
        Trail::Value(v) => {
            let shown = fit(ui, v, 13.0, text_w.min(140.0));
            ui.painter().text(Pos2::new(rect.right() - phone::MARGIN - 12.0, rect.center().y), Align2::RIGHT_CENTER, shown, FontId::proportional(13.0), colour);
        }
    }
    if enabled && r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    let clicked = enabled && r.clicked() && !toggled;
    (clicked, toggled)
}

/// One row card at `y`: the surface and the row. Returns (its height, clicked, toggled).
pub fn row_card(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug + Copy,
    x0: f32,
    y: f32,
    w: f32,
    icon: Option<&'static str>,
    title: &str,
    sub: &str,
    trail: Trail<'_>,
    enabled: bool,
) -> (f32, bool, bool) {
    let h = row_height(ui, sub, w);
    let rect = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, h));
    glass::flat_at(ui, rect, RADIUS, 1.0);
    let (clicked, toggled) = row_at(ui, id, rect, icon, title, sub, trail, enabled);
    (h, clicked, toggled)
}

/// The icon of a row with no art yet: the nearest phone icon.
pub const HAPTIC: &str = icons::TAP_SINGLE;
