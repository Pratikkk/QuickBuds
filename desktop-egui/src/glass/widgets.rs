//! The glass controls. Each `*_at` function draws at a rectangle and returns its `Response`; the page lays them out.

use super::shader;
use eframe::egui::{self, load::Bytes, Align2, Color32, Context, FontId, Id, ImageSource, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};

pub fn white(a: f32) -> Color32 { Color32::from_white_alpha((a * 255.0) as u8) }

/// 1 = the midnight glow behind the glass, 0 = pitch black (the Colorful background switch).
pub fn set_glow(ctx: &Context, glow: f32) { ctx.data_mut(|d| d.insert_temp(Id::new("glass-glow"), glow)); }

fn glow(ctx: &Context) -> f32 { ctx.data(|d| d.get_temp(Id::new("glass-glow"))).unwrap_or(1.0) }

/// The window backdrop. Call first in the central panel.
pub fn backdrop(ui: &Ui) {
    ui.painter().add(shader::backdrop(ui.ctx(), ui.max_rect(), glow(ui.ctx())));
}

/// A glass surface (fill strength `k`).
pub fn glass_at(ui: &Ui, rect: Rect, radius: f32, k: f32) {
    ui.painter().add(shader::surface(ui.ctx(), rect, radius, k, glow(ui.ctx())));
}

/// A glass card at `rect` with content laid out inside it (top-left, `pad` points in).
pub fn card_at<R>(ui: &mut Ui, rect: Rect, radius: f32, pad: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    glass_at(ui, rect, radius, 1.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink(pad)), add).inner
}

pub fn text(ui: &Ui, pos: Pos2, align: Align2, s: &str, size: f32, alpha: f32) {
    ui.painter().text(pos, align, s, FontId::proportional(size), white(alpha));
}

/// An SVG icon (white in the file), tinted.
pub fn icon_at(ui: &Ui, svg: &'static str, rect: Rect, tint: Color32) {
    let uri = format!("bytes://icon-{:p}.svg", svg.as_ptr());
    egui::Image::new(ImageSource::Bytes { uri: uri.into(), bytes: Bytes::Static(svg.as_bytes()) }).tint(tint).paint_at(ui, rect);
}

fn wid(ui: &Ui, id: impl std::hash::Hash + std::fmt::Debug) -> Id { ui.id().with(Id::new(id)) }

fn square(c: Pos2, s: f32) -> Rect { Rect::from_center_size(c, Vec2::splat(s)) }

/// How far a control is hovered (0..1) and pressed (0..1), eased.
fn feel(ui: &Ui, r: &Response) -> (f32, f32) {
    let h = ui.ctx().animate_bool_with_time(r.id.with("h"), r.hovered(), 0.12);
    let p = ui.ctx().animate_bool_with_time(r.id.with("p"), r.is_pointer_button_down_on(), 0.06);
    (h, p)
}

/// A glass pill or circle button. The caller draws the content over `rect`. `selected` lights it white.
pub fn button_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, rect: Rect, radius: f32, selected: bool) -> Response {
    let r = ui.interact(rect, wid(ui, id), Sense::click());
    let (h, p) = feel(ui, &r);
    glass_at(ui, rect, radius, 1.0 + 0.5 * h + 0.5 * p);
    if selected {
        ui.painter().rect_filled(rect.shrink(0.5), radius.min(rect.height() / 2.0), Color32::from_white_alpha(230));
    }
    if r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    r
}

/// A round icon button.
pub fn icon_button_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, rect: Rect, svg: &'static str) -> Response {
    let r = button_at(ui, id, rect, rect.height() / 2.0, false);
    icon_at(ui, svg, square(rect.center(), 22.0), Color32::WHITE);
    r
}

/// A mode disc with its label under it (the noise control). The selected one is lit white with a dark icon.
pub fn disc_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, center: Pos2, diameter: f32, svg: &'static str, label: &str, selected: bool) -> Response {
    let rect = square(center, diameter);
    let hit = Rect::from_min_max(rect.min - Vec2::new(8.0, 0.0), Pos2::new(rect.max.x + 8.0, rect.max.y + 22.0));
    let r = ui.interact(hit, wid(ui, id), Sense::click());
    let (h, p) = feel(ui, &r);
    glass_at(ui, rect, diameter / 2.0, 1.0 + 0.5 * h + 0.5 * p);
    if selected { ui.painter().circle_filled(center, diameter / 2.0 - 0.5, Color32::from_white_alpha(230)); }
    icon_at(ui, svg, square(center, 24.0), if selected { Color32::from_rgb(27, 26, 69) } else { Color32::WHITE });
    text(ui, center + Vec2::new(0.0, diameter / 2.0 + 14.0), Align2::CENTER_CENTER, label, 11.5, if selected { 1.0 } else { 0.8 });
    if r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    r
}

/// The switch: a dark track (light when on), a white thumb that grows and turns glass while pressed. Returns true when toggled.
pub fn switch_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, pos: Pos2, on: bool, enabled: bool) -> bool {
    let rect = Rect::from_min_size(pos, Vec2::new(52.0, 30.0));
    let r = ui.interact(rect, wid(ui, id), Sense::click());
    let t = ui.ctx().animate_bool_with_time(r.id.with("on"), on, 0.14);
    let (_, p) = feel(ui, &r);
    let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    let track = Color32::from_rgba_unmultiplied(lerp(10, 236), lerp(10, 240), lerp(40, 255), lerp(77, 204));
    ui.painter().rect_filled(rect, 15.0, track);
    let x = rect.left() + 15.0 + (rect.width() - 30.0) * t;
    let c = Pos2::new(x, rect.center().y);
    let big = 13.0 + 3.0 * p;
    ui.painter().circle_filled(c + Vec2::new(0.0, 1.5), big, Color32::from_black_alpha(60));
    ui.painter().circle_filled(c, big, Color32::from_white_alpha((255.0 - 80.0 * p) as u8));
    ui.painter().circle_stroke(c, big - 0.5, Stroke::new(1.0, white(0.9 * p)));
    if enabled && r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    enabled && r.clicked()
}

/// A sidebar entry: an icon and a label, lit when selected.
pub fn nav_item_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, rect: Rect, svg: &'static str, label: &str, selected: bool, ready: bool) -> Response {
    let r = ui.interact(rect, wid(ui, id), Sense::click());
    let (h, _) = feel(ui, &r);
    let fill = if selected { 0.26 } else { 0.10 * h };
    if fill > 0.0 { ui.painter().rect_filled(rect, rect.height() / 2.0, white(fill)); }
    let a = if !ready { 0.4 } else if selected { 1.0 } else { 0.8 };
    icon_at(ui, svg, square(rect.left_center() + Vec2::new(25.0, 0.0), 22.0), white(a));
    text(ui, rect.left_center() + Vec2::new(50.0, 0.0), Align2::LEFT_CENTER, label, 15.0, a);
    if ready && r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    r
}

/// What a list row shows on its right.
pub enum Trailing { None, Chevron, Switch(bool), Value(String) }

/// A list row inside a card: icon, title, subtitle, trailing control. 72 points tall. Returns (row clicked, switch toggled).
pub fn row_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug + Copy, rect: Rect, svg: &'static str, title: &str, sub: &str, trailing: Trailing, enabled: bool) -> (bool, bool) {
    let r = ui.interact(rect, wid(ui, id), Sense::click());
    let (h, _) = feel(ui, &r);
    if h > 0.0 && enabled { ui.painter().rect_filled(rect, 0.0, white(0.06 * h)); }
    let a = if enabled { 1.0 } else { 0.5 };
    icon_at(ui, svg, square(rect.left_center() + Vec2::new(34.0, 0.0), 24.0), white(0.88 * a));
    let dy = if sub.is_empty() { 0.0 } else { 11.0 };
    text(ui, rect.left_center() + Vec2::new(58.0, -dy), Align2::LEFT_CENTER, title, 16.0, a);
    if !sub.is_empty() { text(ui, rect.left_center() + Vec2::new(58.0, 10.0), Align2::LEFT_CENTER, sub, 12.5, 0.66 * a); }
    let mut toggled = false;
    match trailing {
        Trailing::None => {}
        Trailing::Chevron => icon_at(ui, crate::icons::CHEVRON_RIGHT, square(rect.right_center() - Vec2::new(32.0, 0.0), 22.0), white(0.7)),
        Trailing::Switch(on) => toggled = switch_at(ui, (id, "sw"), rect.right_center() - Vec2::new(72.0, 15.0), on, enabled),
        Trailing::Value(v) => text(ui, rect.right_center() - Vec2::new(20.0, 0.0), Align2::RIGHT_CENTER, &v, 13.0, 0.7),
    }
    if enabled && r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    (enabled && r.clicked() && !toggled, toggled)
}

/// A hairline between rows.
pub fn divider_at(ui: &Ui, x0: f32, x1: f32, y: f32) {
    ui.painter().hline(x0..=x1, y, Stroke::new(1.0, white(0.16)));
}

/// A battery ring: the track, the level arc (None = unknown), and a glyph in the middle.
pub fn ring_at(ui: &Ui, center: Pos2, radius: f32, frac: Option<f32>, glyph: Option<&'static str>, dim: bool) {
    let a = if dim { 0.4 } else { 1.0 };
    ui.painter().circle_stroke(center, radius, Stroke::new(5.0, white(0.22 * a)));
    if let Some(f) = frac.filter(|f| *f > 0.0) {
        let f = f.min(1.0);
        let n = (f * 96.0) as usize + 2;
        let pts: Vec<Pos2> = (0..=n).map(|i| {
            let ang = -std::f32::consts::FRAC_PI_2 + f * std::f32::consts::TAU * i as f32 / n as f32;
            center + Vec2::angled(ang) * radius
        }).collect();
        ui.painter().add(Shape::line(pts, Stroke::new(5.0, Color32::from_rgba_unmultiplied(232, 236, 255, (255.0 * a) as u8))));
    }
    if let Some(g) = glyph { icon_at(ui, g, square(center, radius * 0.8), white(a)); }
}

/// A slider: a glass track with a white fill and a thumb. Returns true while the value changes.
pub fn slider_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, rect: Rect, value: &mut f32, min: f32, max: f32) -> bool {
    let r = ui.interact(rect, wid(ui, id), Sense::click_and_drag());
    let mut changed = false;
    if let Some(p) = r.interact_pointer_pos().filter(|_| r.is_pointer_button_down_on() || r.dragged()) {
        let t = ((p.x - rect.left() - 12.0) / (rect.width() - 24.0)).clamp(0.0, 1.0);
        let v = min + (max - min) * t;
        if (v - *value).abs() > f32::EPSILON { *value = v; changed = true; }
    }
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let track = Rect::from_center_size(rect.center(), Vec2::new(rect.width() - 8.0, 8.0));
    ui.painter().rect_filled(track, 4.0, white(0.22));
    let x = rect.left() + 12.0 + (rect.width() - 24.0) * t;
    ui.painter().rect_filled(Rect::from_min_max(track.min, Pos2::new(x, track.max.y)), 4.0, white(0.85));
    let (_, p) = feel(ui, &r);
    ui.painter().circle_filled(Pos2::new(x, rect.center().y), 11.0 + 2.0 * p, Color32::WHITE);
    changed
}

/// A dialog: a dimmed window and a centred glass card. `content` gets the card's inner ui. Returns false when closed.
pub fn dialog(ctx: &Context, id: &str, title: &str, size: Vec2, content: impl FnOnce(&mut Ui)) -> bool {
    let screen = ctx.content_rect();
    let mut open = true;
    egui::Area::new(Id::new(id).with("scrim")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
        let r = ui.allocate_rect(screen, Sense::click());
        ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(140));
        if r.clicked() { open = false; }
    });
    let rect = Rect::from_center_size(screen.center(), size);
    egui::Area::new(Id::new(id)).order(egui::Order::Tooltip).fixed_pos(rect.min).show(ctx, |ui| {
        ui.set_clip_rect(screen);
        let (_, resp) = ui.allocate_exact_size(size, Sense::click());
        let _ = resp;
        glass_at(ui, rect, 28.0, 1.6);
        text(ui, rect.min + Vec2::new(24.0, 32.0), Align2::LEFT_CENTER, title, 18.0, 1.0);
        ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_max(rect.min + Vec2::new(24.0, 60.0), rect.max - Vec2::splat(24.0))), content);
    });
    open
}
