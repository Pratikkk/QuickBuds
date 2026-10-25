//! The phone's page geometry (DESIGN §2.1) and the blocks that a settings page shares: a section label, a list row,
//! a chevron and a wrapped paragraph. Sizes are the phone's dp values at 1:1.

use crate::glass;
use crate::icons;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Ui, Vec2};

/// The phone screen is 411.4 dp wide; the desktop shows it as one 412 dp column.
pub const W: f32 = 412.0;
pub const MARGIN: f32 = 16.0;
/// A section label box.
pub const SECTION: f32 = 41.9;
/// A segmented control.
pub const SEGMENT: f32 = 43.8;
/// A list row.
pub const ROW: f32 = 59.8;
/// The gap between two cards on the Themes page.
pub const GAP: f32 = 18.0;
pub const RADIUS: f32 = 24.0;

/// The width of the phone's content column: 379.4 dp, or less in a narrow window.
pub fn content(body: Rect) -> f32 { (W - 2.0 * MARGIN).min(body.width()) }

/// A section label: 14 sp in the accent, in a 41.9 dp box.
pub fn label(ui: &Ui, x: f32, y: f32, text: &str, accent: Color32) {
    ui.painter().text(Pos2::new(x, y + SECTION / 2.0), Align2::LEFT_CENTER, text, FontId::proportional(14.0), accent);
}

/// A chevron: right, or down when the row is open.
pub fn chevron(ui: &Ui, centre: Pos2, open: bool, colour: Color32) {
    let svg = if open { icons::CHEVRON_DOWN } else { icons::CHEVRON_RIGHT };
    glass::icon_at(ui, svg, Rect::from_center_size(centre, Vec2::splat(24.0)), colour);
}

/// What a row shows on its right.
#[derive(Clone, Copy)]
pub enum Trail { None, Chevron, /// An open row: the chevron points down.
    ChevronOpen, Switch(bool) }

/// A row of a settings card.
#[derive(Clone, Copy)]
pub struct Row<'a> {
    pub title: &'a str,
    pub sub: &'a str,
    pub icon: Option<&'static str>,
    /// A colour circle in place of the icon.
    pub swatch: Option<Color32>,
    pub trail: Trail,
    /// The accent: the chevron and the swatch use it.
    pub accent: Color32,
}

impl<'a> Row<'a> {
    pub fn new(title: &'a str, sub: &'a str, trail: Trail, accent: Color32) -> Self {
        Row { title, sub, icon: None, swatch: None, trail, accent }
    }
    pub fn icon(mut self, svg: &'static str) -> Self { self.icon = Some(svg); self }
    pub fn swatch(mut self, c: Color32) -> Self { self.swatch = Some(c); self }
}

/// Cuts a line with an ellipsis before the trailing control. The width is measured, not estimated.
fn fit(ui: &Ui, s: &str, size: f32, width: f32) -> String {
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

/// A row of 59.8 dp: an icon or swatch, a 16 sp title, a 13 sp subtitle, and a chevron or a switch.
/// Returns (row clicked, switch toggled).
pub fn row_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug + Copy, rect: Rect, row: Row, enabled: bool) -> (bool, bool) {
    let r = ui.interact(rect, ui.scope_id().with(id), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(r.id.with("h"), r.hovered() && enabled, 0.12);
    if hover > 0.0 { ui.painter().rect_filled(rect.shrink2(Vec2::new(8.0, 4.0)), 20.0, glass::white(0.07 * hover)); }
    let a = if enabled { 1.0 } else { 0.5 };
    let mut x = rect.left() + MARGIN;
    let mut lead = 0.0;
    if let Some(c) = row.swatch {
        ui.painter().circle_filled(Pos2::new(x + 14.85, rect.center().y), 14.85, c);
        lead = 29.7;
    } else if let Some(svg) = row.icon {
        glass::icon_at(ui, svg, Rect::from_center_size(Pos2::new(x + 12.0, rect.center().y), Vec2::splat(24.0)), glass::white(0.9 * a));
        lead = 24.0;
    }
    x += lead + if lead > 0.0 { 13.7 } else { 0.0 };
    let trailing = match row.trail {
        Trail::Switch(_) => 52.0 + MARGIN,
        Trail::Chevron | Trail::ChevronOpen => 24.0 + 2.0 * MARGIN,
        Trail::None => 0.0,
    };
    let text_w = (rect.right() - trailing - x).max(40.0);
    glass::text(ui, Pos2::new(x, rect.top() + 19.6), Align2::LEFT_CENTER, &fit(ui, row.title, 16.0, text_w), 16.0, a);
    if !row.sub.is_empty() {
        glass::text(ui, Pos2::new(x, rect.top() + 41.5), Align2::LEFT_CENTER, &fit(ui, row.sub, 13.0, text_w), 13.0, 0.74 * a);
    }
    let mut toggled = false;
    match row.trail {
        Trail::None => {}
        Trail::Chevron | Trail::ChevronOpen => chevron(ui, Pos2::new(rect.right() - MARGIN - 12.0, rect.center().y), matches!(row.trail, Trail::ChevronOpen), Color32::from_rgba_unmultiplied(row.accent.r(), row.accent.g(), row.accent.b(), (255.0 * a) as u8)),
        Trail::Switch(on) => toggled = glass::switch_at(ui, (id, "sw"), Pos2::new(rect.right() - MARGIN - 52.0, rect.center().y - 16.0), on, enabled),
    }
    if enabled && r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    (enabled && r.clicked() && !toggled, toggled)
}

/// The height of a wrapped paragraph at `size` in a box `width` wide.
pub fn wrap_height(ui: &Ui, text: &str, size: f32, width: f32) -> f32 {
    ui.painter().layout(text.to_string(), FontId::proportional(size), Color32::WHITE, width).size().y
}

/// A wrapped paragraph in `rect`.
pub fn paragraph(ui: &mut Ui, rect: Rect, text: &str, size: f32, colour: Color32, centred: bool) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        let layout = egui::Layout::top_down(if centred { egui::Align::Center } else { egui::Align::Min });
        ui.with_layout(layout, |ui| {
            ui.add(egui::Label::new(egui::RichText::new(text).size(size).color(colour)).wrap());
        });
    });
}
