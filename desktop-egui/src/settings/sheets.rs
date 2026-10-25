//! The phone's floating sheet (DESIGN §4.9, Glass): 34 dp corners, a 0.45 scrim, a 17 sp title and 50 dp buttons.
//! The card is 363.4 dp wide with a 24 dp screen inset.

use crate::glass;
use crate::motion;
use eframe::egui::{self, Align2, Context, FontId, Id, Pos2, Rect, Sense, Ui, Vec2};

/// The width of the phone's floating sheet.
pub const WIDTH: f32 = 363.4;

/// Where the title sits.
#[derive(Clone, Copy)]
pub enum Title { Centre, Start }

/// A sheet with a title and a content area. Returns false when the user taps outside it.
///
/// `content` gets the inner rectangle: 24 dp side padding, below the title, above the 16 dp bottom pad.
pub fn sheet(ctx: &Context, id: &str, title: &str, align: Title, size: Vec2, content: impl FnOnce(&mut Ui, Rect)) -> bool {
    let screen = ctx.content_rect();
    let mut open = true;
    let m = motion::sheet_enter(ctx, Id::NULL.with(id).with("in"));
    let a = m.alpha;
    egui::Area::new(Id::NULL.with(id).with("scrim")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
        let r = ui.allocate_rect(screen, Sense::click());
        ui.painter().rect_filled(screen, 0.0, egui::Color32::from_black_alpha((0.45 * 255.0 * a) as u8));
        if r.clicked() { open = false; }
    });
    let rect = Rect::from_center_size(screen.center(), size * m.scale);
    egui::Area::new(Id::NULL.with(id)).order(egui::Order::Tooltip).fixed_pos(rect.min).show(ctx, |ui| {
        ui.set_opacity(a);
        ui.set_clip_rect(screen);
        ui.allocate_exact_size(size, Sense::click());
        glass::glass_at(ui, rect, 34.0, 1.6);
        let (pos, anchor) = match align {
            Title::Centre => (Pos2::new(rect.center().x, rect.top() + 36.0), Align2::CENTER_CENTER),
            Title::Start => (Pos2::new(rect.left() + 31.0, rect.top() + 36.0), Align2::LEFT_CENTER),
        };
        ui.painter().text(pos, anchor, title, FontId::proportional(17.0), glass::white(1.0));
        let inner = Rect::from_min_max(
            Pos2::new(rect.left() + 14.0, rect.top() + 74.3),
            Pos2::new(rect.right() - 14.0, rect.bottom() - 16.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| content(ui, inner));
    });
    open
}

/// The button row at the bottom of a sheet's inner rectangle. `cancel` is left, `accept` right.
/// An empty label takes no space, so one button fills the row. Some(true) = accept, Some(false) = cancel.
pub fn actions(ui: &mut Ui, inner: Rect, cancel: &str, accept: &str) -> Option<bool> {
    let y = inner.bottom() - 50.0;
    let h = 50.0;
    let (left, right) = (inner.left() + 10.0, inner.right() - 10.0);
    let mut out = None;
    match (cancel.is_empty(), accept.is_empty()) {
        (true, true) => {}
        (false, true) => {
            let r = Rect::from_min_size(Pos2::new(left, y), Vec2::new(right - left, h));
            if glass::button_at(ui, "sheet-cancel", r, h / 2.0, false).clicked() { out = Some(false); }
            ui.painter().text(r.center(), Align2::CENTER_CENTER, cancel, FontId::proportional(15.0), glass::white(1.0));
        }
        (true, false) => {
            let r = Rect::from_min_size(Pos2::new(left, y), Vec2::new(right - left, h));
            if glass::button_at(ui, "sheet-ok", r, h / 2.0, false).clicked() { out = Some(true); }
            ui.painter().rect_filled(r, h / 2.0, glass::style::accent());
            ui.painter().text(r.center(), Align2::CENTER_CENTER, accept, FontId::proportional(15.0), glass::on_accent());
        }
        (false, false) => {
            let w = (right - left - 8.0) / 2.0;
            let can = Rect::from_min_size(Pos2::new(left, y), Vec2::new(w, h));
            if glass::button_at(ui, "sheet-cancel", can, h / 2.0, false).clicked() { out = Some(false); }
            ui.painter().text(can.center(), Align2::CENTER_CENTER, cancel, FontId::proportional(15.0), glass::white(1.0));
            let ok = Rect::from_min_size(Pos2::new(left + w + 8.0, y), Vec2::new(w, h));
            if glass::button_at(ui, "sheet-ok", ok, h / 2.0, false).clicked() { out = Some(true); }
            ui.painter().rect_filled(ok, h / 2.0, glass::style::accent());
            ui.painter().text(ok.center(), Align2::CENTER_CENTER, accept, FontId::proportional(15.0), glass::on_accent());
        }
    }
    out
}

/// The height of a row inside a sheet: a 16 sp title that wraps, a wrapped 13 sp subtitle, a switch, 8 dp pads.
pub fn row_height(ui: &Ui, title: &str, sub: &str, width: f32) -> f32 {
    let text_w = (width - 2.0 * 15.7 - 58.0).max(60.0);
    let title_h = crate::phone::wrap_height(ui, title, 16.0, text_w);
    (8.0 + title_h + crate::phone::wrap_height(ui, sub, 13.0, text_w) + 8.0).max(59.8)
}

/// A row inside a sheet: a title that may wrap, a wrapped subtitle and a switch at the end. Returns true when toggled.
pub fn row_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug + Copy, rect: Rect, title: &str, sub: &str, on: bool) -> bool {
    let r = ui.interact(rect, ui.scope_id().with(id), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(r.id.with("h"), r.hovered(), 0.12);
    if hover > 0.0 { ui.painter().rect_filled(rect.shrink2(Vec2::new(6.0, 3.0)), 16.0, glass::white(0.06 * hover)); }
    let x = rect.left() + 15.7;
    let text_w = (rect.right() - 15.7 - 58.0 - x).max(60.0);
    let title_galley = ui.painter().layout(title.to_string(), FontId::proportional(16.0), glass::white(0.9), text_w);
    let title_h = title_galley.size().y;
    ui.painter().galley(Pos2::new(x, rect.top() + 8.0), title_galley, glass::white(0.9));
    let galley = ui.painter().layout(sub.to_string(), FontId::proportional(13.0), glass::white(0.74), text_w);
    ui.painter().galley(Pos2::new(x, rect.top() + 9.0 + title_h), galley, glass::white(0.74));
    if r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    let switched = glass::switch_at(ui, (id, "sw"), Pos2::new(rect.right() - 15.7 - 52.0, rect.center().y - 16.0), on, true);
    switched || r.clicked()
}
