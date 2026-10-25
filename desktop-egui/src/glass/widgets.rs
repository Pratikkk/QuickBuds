//! The glass controls. Each `*_at` function draws at a rectangle and returns its `Response`; the page lays them out.

use super::{motion, shader};
use super::style::{self, Style};
use crate::palette::blend;
use eframe::egui::{self, load::Bytes, Align2, Color32, Context, FontId, Id, ImageSource, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};

/// The text and icon colour at strength `a`: white in Glass, the palette's text in the other styles.
pub fn white(a: f32) -> Color32 { style::ink(a) }

/// The colour of a label on a selection's fill.
pub fn on_accent() -> Color32 { style::on_accent() }

/// A selection's fill at strength `a` (0..1): lit glass, the Material tonal colour, or the accent.
pub fn sel_fill(a: f32) -> Color32 {
    let p = style::pal();
    match style::style() {
        Style::Glass => Color32::from_white_alpha((0.26 * 255.0 * a) as u8),
        Style::Material => { let c = p.tonal(); Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (255.0 * a) as u8) }
        _ => { let c = p.accent; Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (0.30 * 255.0 * a) as u8) }
    }
}

/// A strong selection (a lit button or disc): white glass, or the accent.
fn lit(a: f32) -> Color32 {
    if style::glass() { Color32::from_white_alpha((230.0 * a) as u8) } else { let c = style::accent(); Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (255.0 * a) as u8) }
}

/// 1 = the midnight glow behind the glass, 0 = pitch black (the Colorful background switch).
pub fn set_glow(ctx: &Context, glow: f32) { ctx.data_mut(|d| d.insert_temp(Id::NULL.with("glass-glow"), glow)); }

fn glow(ctx: &Context) -> f32 { ctx.data(|d| d.get_temp(Id::NULL.with("glass-glow"))).unwrap_or(1.0) }

/// The window: its pane with rounded corners, the glow (Glass) or the page colour, and a light line on the edge.
/// Call first in the central panel.
pub fn backdrop(ui: &Ui) {
    let ctx = ui.ctx();
    let p = style::pal();
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false));
    let radius = if maximized { 0.0 } else { 26.0 };
    let w = if style::glass() {
        shader::Window { radius, glow: glow(ctx), solid: None, edge: [1.0, 1.0, 1.0, if maximized { 0.0 } else { 0.26 }] }
    } else {
        let f = |c: Color32| [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0];
        let o = f(p.outline);
        shader::Window { radius, glow: 0.0, solid: Some(f(p.bg)), edge: [o[0], o[1], o[2], if maximized { 0.0 } else { 0.9 }] }
    };
    ui.painter().add(shader::window(ctx, ui.max_rect(), w));
}

/// A content surface. Liquid Glass keeps glass for the functional layer (sidebar, buttons, controls, dialogs), so in Glass
/// content is a flat, faintly lit plate with a hairline edge; the other styles draw the same card as `glass_at`.
pub fn flat_at(ui: &Ui, rect: Rect, radius: f32, k: f32) {
    if style::style() != Style::Glass { return glass_at(ui, rect, radius, k); }
    let r = radius.min(rect.height() / 2.0);
    ui.painter().rect_filled(rect, r, white(0.07 + 0.03 * (k - 1.0).clamp(0.0, 1.0)));
    ui.painter().rect_stroke(rect, r, Stroke::new(1.0, white(0.11)), StrokeKind::Inside);
}

/// A surface (fill strength `k`, above 1 on hover): real glass in Glass; a tonal card in Material; a card with an outline in
/// QuickBuds; a card with a dotted outline in Pixel art.
pub fn glass_at(ui: &Ui, rect: Rect, radius: f32, k: f32) {
    let p = style::pal();
    let up = ((k - 1.0) * 0.09).clamp(0.0, 0.25);
    match style::style() {
        Style::Glass => { ui.painter().add(shader::surface(ui.ctx(), rect, radius, k, ui.opacity())); }
        Style::Material => {
            ui.painter().rect_filled(rect, radius.min(rect.height() / 2.0).min(26.0), blend(p.container(), p.text, up));
        }
        Style::Classic => {
            let r = radius.min(rect.height() / 2.0).min(22.0);
            ui.painter().rect_filled(rect, r, blend(p.card, p.text, up));
            ui.painter().rect_stroke(rect, r, Stroke::new(1.0, p.outline), StrokeKind::Inside);
        }
        Style::Pixel => {
            ui.painter().rect_filled(rect, 4.0, blend(p.card, p.text, up));
            let r = rect.shrink(1.0);
            let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
            ui.painter().extend(Shape::dotted_line(&pts, p.text2, 5.0, 1.1));
        }
    }
}

/// A glass card at `rect` with content laid out inside it (top-left, `pad` points in).
pub fn card_at<R>(ui: &mut Ui, rect: Rect, radius: f32, pad: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    flat_at(ui, rect, radius, 1.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink(pad)), add).inner
}

pub fn text(ui: &Ui, pos: Pos2, align: Align2, s: &str, size: f32, alpha: f32) {
    ui.painter().text(pos, align, s, FontId::proportional(size), white(alpha));
}

/// The aspect (width / height) of an icon from its `viewBox`.
fn aspect(svg: &str) -> f32 {
    let vb = svg.find("viewBox=\"").map(|i| &svg[i + 9..]).and_then(|s| s.split('"').next());
    let n: Vec<f32> = vb.map(|v| v.split_whitespace().filter_map(|x| x.parse().ok()).collect()).unwrap_or_default();
    if n.len() == 4 && n[3] > 0.0 { n[2] / n[3] } else { 1.0 }
}

/// An SVG icon (white in the file), tinted, fitted into `rect` with its own aspect (the buds and the case are not square).
pub fn icon_at(ui: &Ui, svg: &'static str, rect: Rect, tint: Color32) {
    let uri = format!("bytes://icon-{:p}.svg", svg.as_ptr());
    let a = aspect(svg);
    let size = if a >= rect.width() / rect.height() { Vec2::new(rect.width(), rect.width() / a) } else { Vec2::new(rect.height() * a, rect.height()) };
    let fitted = Rect::from_center_size(rect.center(), size);
    egui::Image::new(ImageSource::Bytes { uri: uri.into(), bytes: Bytes::Static(svg.as_bytes()) }).tint(tint).paint_at(ui, fitted);
}

fn wid(ui: &Ui, id: impl std::hash::Hash + std::fmt::Debug) -> Id { ui.scope_id().with(egui::IdSalt::new(id)) }

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
        ui.painter().rect_filled(rect.shrink(0.5), radius.min(rect.height() / 2.0), lit(1.0));
    }
    if r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    r
}

/// A round icon button.
pub fn icon_button_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, rect: Rect, svg: &'static str) -> Response {
    let r = button_at(ui, id, rect, rect.height() / 2.0, false);
    icon_at(ui, svg, square(rect.center(), 22.0), white(1.0));
    r
}

/// A mode disc with its label under it (the noise control). The selected one is lit white with a dark icon.
pub fn disc_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, center: Pos2, diameter: f32, svg: &'static str, label: &str, selected: bool) -> Response {
    let rect = square(center, diameter);
    let hit = Rect::from_min_max(rect.min - Vec2::new(8.0, 0.0), Pos2::new(rect.max.x + 8.0, rect.max.y + 22.0));
    let r = ui.interact(hit, wid(ui, id), Sense::click());
    let (h, p) = feel(ui, &r);
    // The lit disc fades in and out as the selection moves.
    let t = ui.ctx().animate_bool_with_time(r.id.with("sel"), selected, 0.2);
    glass_at(ui, rect, diameter / 2.0, 1.0 + 0.5 * h + 0.5 * p);
    if t > 0.0 {
        if style::glass() {
            // Lit glass, not a white plate: a halo, a bright body, a rim; the icon stays white.
            ui.painter().circle_filled(center, diameter / 2.0 + 3.0, white(0.10 * t));
            ui.painter().circle_filled(center, diameter / 2.0 - 0.5, white(0.36 * t));
            ui.painter().circle_filled(center + Vec2::new(0.0, diameter * 0.18), diameter / 2.0 * 0.6, Color32::from_rgba_unmultiplied(184, 198, 255, (46.0 * t) as u8));
            ui.painter().circle_stroke(center, diameter / 2.0 - 0.6, Stroke::new(1.2, white(0.75 * t)));
        } else { ui.painter().circle_filled(center, diameter / 2.0 - 0.5, lit(t)); }
    }
    let (from, to) = (white(1.0), if style::glass() { white(1.0) } else { on_accent() });
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    icon_at(ui, svg, square(center, 24.0), Color32::from_rgb(mix(from.r(), to.r()), mix(from.g(), to.g()), mix(from.b(), to.b())));
    text(ui, center + Vec2::new(0.0, diameter / 2.0 + 14.0), Align2::CENTER_CENTER, label, 11.5, 0.8 + 0.2 * t);
    if r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    r
}

/// The switch. Glass: a dark track (light when on) and a white thumb that grows and turns glass while pressed. Material: the
/// M3 switch. QuickBuds and Pixel art: an accent track. Returns true when toggled.
///
/// The track fades in 160 ms (DESIGN row 13). The thumb slides in 155 ms (row 14). The first value
/// from the buds snaps (row 15). A later real change slides. A tap returns true and does not move the
/// switch, because the caller sends the request and the buds' report moves the control.
pub fn switch_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, pos: Pos2, on: bool, enabled: bool) -> bool {
    let (w, h) = match style::style() { Style::Material => (52.0, 32.0), _ => (52.0, 30.0) };
    let rect = Rect::from_min_size(pos, Vec2::new(w, h));
    let r = ui.interact(rect, wid(ui, id), Sense::click());
    let key = r.id;
    let track = motion::fade(ui.ctx(), key.with("track"), on, motion::SWITCH_TRACK);
    let target = if on { 1.0 } else { 0.0 };
    let t = motion::tween(ui.ctx(), key.with("slide"), target, motion::SWITCH_SLIDE, motion::SWITCH_SLIDE);
    let (_, p) = feel(ui, &r);
    let pal = style::pal();
    match style::style() {
        Style::Glass => {
            let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * track) as u8;
            let track_col = Color32::from_rgba_unmultiplied(lerp(10, 236), lerp(10, 240), lerp(40, 255), lerp(77, 204));
            ui.painter().rect_filled(rect, 15.0, track_col);
            let x = rect.left() + 15.0 + (rect.width() - 30.0) * t;
            let c = Pos2::new(x, rect.center().y);
            let big = 13.0 + 3.0 * p;
            ui.painter().circle_filled(c + Vec2::new(0.0, 1.5), big, Color32::from_black_alpha(60));
            ui.painter().circle_filled(c, big, Color32::from_white_alpha((255.0 - 80.0 * p) as u8));
            ui.painter().circle_stroke(c, big - 0.5, Stroke::new(1.0, Color32::from_white_alpha((230.0 * p) as u8)));
        }
        Style::Material => {
            ui.painter().rect_filled(rect, 16.0, blend(pal.container(), pal.accent, track));
            if track < 1.0 { ui.painter().rect_stroke(rect, 16.0, Stroke::new(2.0 * (1.0 - track), pal.text2), StrokeKind::Inside); }
            let c = Pos2::new(rect.left() + 16.0 + (rect.width() - 32.0) * t, rect.center().y);
            let rad = (8.0 + 4.0 * t + 2.0 * p) as f32;
            ui.painter().circle_filled(c, rad, blend(pal.text2, pal.on_accent(), track));
        }
        s => {
            let round = if s == Style::Pixel { 6.0 } else { 15.0 };
            ui.painter().rect_filled(rect, round, blend(pal.track(), pal.accent, track));
            let c = Pos2::new(rect.left() + 15.0 + (rect.width() - 30.0) * t, rect.center().y);
            let thumb = if pal.light() { pal.card } else { blend(pal.text, pal.card, 0.1) };
            if s == Style::Pixel { ui.painter().rect_filled(Rect::from_center_size(c, Vec2::splat(22.0 + 2.0 * p)), 4.0, thumb); }
            else { ui.painter().circle_filled(c, 11.0 + 2.0 * p, thumb); }
        }
    }
    if enabled && r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    enabled && r.clicked()
}

/// A sidebar entry: an icon and a label, lit when selected.
pub fn nav_item_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, rect: Rect, svg: &'static str, label: &str, selected: bool, ready: bool) -> Response {
    let r = ui.interact(rect, wid(ui, id), Sense::click());
    let (h, _) = feel(ui, &r);
    let fill = if selected { 0.0 } else { 0.10 * h };
    if fill > 0.0 { ui.painter().rect_filled(rect, rect.height() / 2.0, white(fill)); }
    let a = if !ready { 0.4 } else if selected { 1.0 } else { 0.8 };
    icon_at(ui, svg, square(rect.left_center() + Vec2::new(25.0, 0.0), 22.0), white(a));
    text(ui, rect.left_center() + Vec2::new(50.0, 0.0), Align2::LEFT_CENTER, label, 15.0, a);
    if ready && r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    r
}

/// What a list row shows on its right.
pub enum Trailing { None, Chevron, Switch(bool), Value(String) }

/// A home tile: icon top left, a switch or chevron top right, the title and a line under it at the bottom.
/// Returns (tile clicked, switch toggled).
pub fn tile_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug + Copy, rect: Rect, svg: &'static str, title: &str, sub: &str, trailing: Trailing, enabled: bool) -> (bool, bool) {
    let r = ui.interact(rect, wid(ui, id), Sense::click());
    let (h, p) = feel(ui, &r);
    flat_at(ui, rect, 28.0, 1.0 + 0.5 * h * if enabled { 1.0 } else { 0.0 } + 0.3 * p);
    let a = if enabled { 1.0 } else { 0.5 };
    icon_at(ui, svg, square(rect.left_top() + Vec2::new(36.0, 36.0), 26.0), white(0.9 * a));
    let mut toggled = false;
    match &trailing {
        Trailing::Switch(on) => { toggled = switch_at(ui, (id, "sw"), Pos2::new(rect.right() - 22.0 - 52.0, rect.top() + 20.0), *on, enabled); }
        Trailing::Chevron => icon_at(ui, crate::icons::CHEVRON_RIGHT, square(rect.right_top() + Vec2::new(-30.0, 34.0), 18.0), white(0.7 * a)),
        _ => {}
    }
    // The lines are cut with an ellipsis when the tile is narrow.
    let fit = |s: &str, size: f32| -> String {
        let max = ((rect.width() - 40.0) / (size * 0.52)) as usize;
        if s.chars().count() <= max { s.to_string() } else { format!("{}…", s.chars().take(max.saturating_sub(1)).collect::<String>().trim_end()) }
    };
    let dy = if sub.is_empty() { 0.0 } else { 12.0 };
    text(ui, Pos2::new(rect.left() + 22.0, rect.bottom() - 28.0 - dy), Align2::LEFT_CENTER, &fit(title, 16.5), 16.5, a);
    if !sub.is_empty() { text(ui, Pos2::new(rect.left() + 22.0, rect.bottom() - 24.0), Align2::LEFT_CENTER, &fit(sub, 12.5), 12.5, 0.74 * a); }
    if enabled && r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    // The switch takes its own clicks; a click anywhere else on a switch tile toggles it too.
    if matches!(trailing, Trailing::Switch(_)) { (false, toggled || (r.clicked() && enabled)) } else { (r.clicked(), false) }
}

/// A list row inside a card: icon, title, subtitle, trailing control. 72 points tall. Returns (row clicked, switch toggled).
pub fn row_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug + Copy, rect: Rect, svg: &'static str, title: &str, sub: &str, trailing: Trailing, enabled: bool) -> (bool, bool) {
    let r = ui.interact(rect, wid(ui, id), Sense::click());
    let (h, _) = feel(ui, &r);
    // The hover plate sits inside the card with round ends, so it never pokes out of a rounded corner.
    if h > 0.0 && enabled { ui.painter().rect_filled(rect.shrink2(Vec2::new(8.0, 5.0)), 18.0, white(0.07 * h)); }
    let a = if enabled { 1.0 } else { 0.5 };
    icon_at(ui, svg, square(rect.left_center() + Vec2::new(34.0, 0.0), 24.0), white(0.88 * a));
    let dy = if sub.is_empty() { 0.0 } else { 11.0 };
    text(ui, rect.left_center() + Vec2::new(58.0, -dy), Align2::LEFT_CENTER, title, 16.0, a);
    if !sub.is_empty() {
        // Cut with an ellipsis before the trailing control.
        let max = ((rect.width() - 58.0 - 84.0) / 6.4) as usize;
        let shown = if sub.chars().count() <= max { sub.to_string() } else { format!("{}…", sub.chars().take(max.saturating_sub(1)).collect::<String>().trim_end()) };
        text(ui, rect.left_center() + Vec2::new(58.0, 10.0), Align2::LEFT_CENTER, &shown, 12.5, 0.74 * a);
    }
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

/// A one-line list item (a preset): lit inside the card when selected, dimmed while it waits for the buds.
pub fn list_item_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, rect: Rect, label: &str, selected: bool, dim: bool) -> Response {
    let r = ui.interact(rect, wid(ui, id), Sense::click());
    let (h, _) = feel(ui, &r);
    let t = ui.ctx().animate_bool_with_time(r.id.with("sel"), selected, 0.18);
    let inner = rect.shrink2(Vec2::new(8.0, 5.0));
    if t > 0.0 { ui.painter().rect_filled(inner, 20.0, sel_fill(t)); }
    else if h > 0.0 { ui.painter().rect_filled(inner, 20.0, white(0.08 * h)); }
    text(ui, rect.left_center() + Vec2::new(28.0, 0.0), Align2::LEFT_CENTER, label, 16.0, if dim { 0.5 } else { 1.0 });
    if t > 0.0 { ui.painter().circle_filled(rect.right_center() - Vec2::new(64.0, 0.0), 5.0 * t, white(t)); }
    if r.hovered() && !dim { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    r
}

/// A hairline between rows.
pub fn divider_at(ui: &Ui, x0: f32, x1: f32, y: f32) {
    ui.painter().hline(x0..=x1, y, Stroke::new(1.0, white(0.16)));
}

/// A gradient arc as a mesh: `c0` at the start to `c1` at the end, round caps.
fn arc(ui: &Ui, center: Pos2, radius: f32, width: f32, frac: f32, c0: Color32, c1: Color32) {
    let n = (frac * 360.0).ceil().max(2.0) as usize;
    let mut mesh = egui::epaint::Mesh::default();
    let lerp = |t: f32| Color32::from_rgba_unmultiplied(
        (c0.r() as f32 + (c1.r() as f32 - c0.r() as f32) * t) as u8,
        (c0.g() as f32 + (c1.g() as f32 - c0.g() as f32) * t) as u8,
        (c0.b() as f32 + (c1.b() as f32 - c0.b() as f32) * t) as u8,
        (c0.a() as f32 + (c1.a() as f32 - c0.a() as f32) * t) as u8);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let ang = -std::f32::consts::FRAC_PI_2 + frac * std::f32::consts::TAU * t;
        let d = Vec2::angled(ang);
        let col = lerp(t);
        // Four vertices per step: a transparent fringe outside, the arc, a fringe inside. The fringe anti-aliases the edge.
        let fe = 1.0 / ui.ctx().pixels_per_point();
        mesh.colored_vertex(center + d * (radius - width / 2.0 - fe), Color32::TRANSPARENT);
        mesh.colored_vertex(center + d * (radius - width / 2.0), col);
        mesh.colored_vertex(center + d * (radius + width / 2.0), col);
        mesh.colored_vertex(center + d * (radius + width / 2.0 + fe), Color32::TRANSPARENT);
        if i > 0 {
            let k = (i * 4) as u32;
            for j in 0..3 {
                mesh.add_triangle(k - 4 + j, k - 3 + j, k + j);
                mesh.add_triangle(k - 3 + j, k + 1 + j, k + j);
            }
        }
    }
    ui.painter().add(Shape::mesh(mesh));
    let end = center + Vec2::angled(-std::f32::consts::FRAC_PI_2 + frac * std::f32::consts::TAU) * radius;
    ui.painter().circle_filled(center + Vec2::angled(-std::f32::consts::FRAC_PI_2) * radius, width / 2.0, c0);
    ui.painter().circle_filled(end, width / 2.0, c1);
}

/// A battery ring in the glass style: a lit glass disc, the track, a soft light along the gradient arc and a glyph.
/// The arc sweeps to each new level (350 ms up, 500 ms down, decelerate) and snaps on the first paint.
/// A connection change cross-fades the ring in 175 ms. Returns the shown fraction, so a percentage can follow the arc.
pub fn ring_at(ui: &Ui, id: impl std::hash::Hash + std::fmt::Debug, center: Pos2, radius: f32, frac: Option<f32>, glyph: Option<&'static str>, dim: bool) -> f32 {
    let key = wid(ui, id);
    let v = motion::tween(ui.ctx(), key.with("v"), frac.unwrap_or(0.0), motion::RING_FILL, motion::RING_EMPTY);
    let a = 1.0 - 0.6 * motion::fade(ui.ctx(), key.with("fade"), dim, motion::RING_FADE);
    let width = 6.0;
    if !style::glass() {
        // The other styles: a plain track and an accent arc; the glyph in the accent.
        let pal = style::pal();
        let fade = |c: Color32| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (255.0 * a) as u8);
        ui.painter().circle_stroke(center, radius, Stroke::new(width, fade(pal.track())));
        if v > 0.004 { arc(ui, center, radius, width, v.min(1.0), fade(pal.accent), fade(pal.accent)); }
        if let Some(g) = glyph {
            let h = radius * 1.05;
            icon_at(ui, g, Rect::from_center_size(center, Vec2::new(h * aspect(g), h)), fade(pal.accent));
        }
        return v;
    }
    // Flat like the rest of the screen: a faint disc, a thin track and a white arc, no light effects.
    let inner = radius - width / 2.0 - 1.0;
    ui.painter().circle_filled(center, inner, white(0.08 * a));
    ui.painter().circle_stroke(center, radius, Stroke::new(width, white(0.18 * a)));
    if v > 0.004 {
        let f = v.min(1.0);
        let col = Color32::from_rgba_unmultiplied(255, 255, 255, (245.0 * a) as u8);
        if f >= 0.995 { ui.painter().circle_stroke(center, radius, Stroke::new(width, col)); } else { arc(ui, center, radius, width, f, col, col); }
    }
    if let Some(g) = glyph {
        let h = radius * 1.05;
        icon_at(ui, g, Rect::from_center_size(center, Vec2::new(h * aspect(g), h)), white(a));
    }
    v
}

/// What a slider did this frame.
#[derive(Default, Clone, Copy)]
pub struct Slide { pub changed: bool, pub released: bool }

fn slide_value(r: &Response, t: f32, min: f32, max: f32, whole: bool) -> Option<f32> {
    r.interact_pointer_pos().filter(|_| r.is_pointer_button_down_on() || r.dragged())?;
    let v = min + (max - min) * t.clamp(0.0, 1.0);
    Some(if whole { v.round() } else { v })
}

/// A horizontal slider: a glass track with a white fill and a thumb.
pub fn slider_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, rect: Rect, value: &mut f32, min: f32, max: f32, whole: bool) -> Slide {
    let r = ui.interact(rect, wid(ui, id), Sense::click_and_drag());
    let mut s = Slide { changed: false, released: r.drag_stopped() || r.clicked() };
    if let Some(p) = r.interact_pointer_pos() {
        if let Some(v) = slide_value(&r, (p.x - rect.left() - 12.0) / (rect.width() - 24.0), min, max, whole) {
            if v != *value { *value = v; s.changed = true; }
        }
    }
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let track = Rect::from_center_size(rect.center(), Vec2::new(rect.width() - 8.0, 8.0));
    ui.painter().rect_filled(track, 4.0, white(0.22));
    let x = rect.left() + 12.0 + (rect.width() - 24.0) * t;
    let fill = if style::glass() { white(0.85) } else { style::accent() };
    ui.painter().rect_filled(Rect::from_min_max(track.min, Pos2::new(x, track.max.y)), 4.0, fill);
    let (_, p) = feel(ui, &r);
    ui.painter().circle_filled(Pos2::new(x, rect.center().y), 11.0 + 2.0 * p, if style::glass() { Color32::WHITE } else { style::accent() });
    s
}

/// A vertical slider (an EQ band): the top is `max`.
pub fn vslider_at(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, rect: Rect, value: &mut f32, min: f32, max: f32, whole: bool) -> Slide {
    let r = ui.interact(rect, wid(ui, id), Sense::click_and_drag());
    let mut s = Slide { changed: false, released: r.drag_stopped() || r.clicked() };
    if let Some(p) = r.interact_pointer_pos() {
        if let Some(v) = slide_value(&r, 1.0 - (p.y - rect.top() - 12.0) / (rect.height() - 24.0), min, max, whole) {
            if v != *value { *value = v; s.changed = true; }
        }
    }
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let track = Rect::from_center_size(rect.center(), Vec2::new(8.0, rect.height() - 8.0));
    ui.painter().rect_filled(track, 4.0, white(0.22));
    let y = rect.bottom() - 12.0 - (rect.height() - 24.0) * t;
    let zero = rect.bottom() - 12.0 - (rect.height() - 24.0) * ((0.0 - min) / (max - min));
    let fill = if style::glass() { white(0.85) } else { style::accent() };
    ui.painter().rect_filled(Rect::from_min_max(Pos2::new(track.left(), y.min(zero)), Pos2::new(track.right(), y.max(zero))), 4.0, fill);
    let (_, p) = feel(ui, &r);
    ui.painter().circle_filled(Pos2::new(rect.center().x, y), 11.0 + 2.0 * p, if style::glass() { Color32::WHITE } else { style::accent() });
    s
}

/// A text field on a glass pill. Returns the field's response; it has `lost_focus()` when editing ends.
pub fn text_field_at(ui: &mut Ui, rect: Rect, text: &mut String, max_len: usize) -> Response {
    glass_at(ui, rect, rect.height() / 2.0, 1.0);
    let inner = rect.shrink2(Vec2::new(18.0, 6.0));
    ui.put(inner, egui::TextEdit::singleline(text).frame(egui::Frame::NONE).char_limit(max_len).text_color(white(1.0)).align(egui::Align2::LEFT_CENTER))
}

/// A list of choices in a dialog. `Pick(i)` on a click, `Close` when dismissed, `Open` otherwise.
pub enum Choice { Open, Pick(usize), Close }

pub fn choice_dialog(ctx: &Context, id: &str, title: &str, options: &[String], current: usize) -> Choice {
    let h = 60.0 + 52.0 * options.len() as f32 + 24.0;
    let mut pick = None;
    let open = dialog(ctx, id, title, Vec2::new(420.0, h.min(ctx.content_rect().height() - 40.0)), |ui| {
        let r = ui.max_rect();
        for (i, o) in options.iter().enumerate() {
            let row = Rect::from_min_size(Pos2::new(r.left() - 12.0, r.top() + 52.0 * i as f32 - 8.0), Vec2::new(r.width() + 24.0, 52.0));
            if list_item_at(ui, ("choice", i), row, o, i == current, false).clicked() { pick = Some(i); }
        }
    });
    match (pick, open) { (Some(i), _) => Choice::Pick(i), (None, true) => Choice::Open, (None, false) => Choice::Close }
}

/// Cancel and Accept at the bottom of a dialog's content area. Some(true) accept, Some(false) cancel.
pub fn dialog_buttons(ui: &mut Ui, cancel: &str, accept: &str) -> Option<bool> {
    let r = ui.max_rect();
    let y = r.bottom() - 44.0;
    let acc = Rect::from_min_size(Pos2::new(r.right() - 130.0, y), Vec2::new(130.0, 44.0));
    let can = Rect::from_min_size(Pos2::new(acc.left() - 10.0 - 110.0, y), Vec2::new(110.0, 44.0));
    let mut out = None;
    if button_at(ui, "cancel", can, 22.0, false).clicked() { out = Some(false); }
    text(ui, can.center(), Align2::CENTER_CENTER, cancel, 15.0, 1.0);
    if !accept.is_empty() {
        if button_at(ui, "accept", acc, 22.0, true).clicked() { out = Some(true); }
        ui.painter().text(acc.center(), Align2::CENTER_CENTER, accept, FontId::proportional(15.0), on_accent());
    }
    out
}

/// A dialog: a dimmed window and a centred glass card. `content` gets the card's inner ui. Returns false when closed.
/// The card and the scrim grow from 92% and fade in over 260 ms with a 1.6 overshoot (DESIGN row 17).
pub fn dialog(ctx: &Context, id: &str, title: &str, size: Vec2, content: impl FnOnce(&mut Ui)) -> bool {
    let screen = ctx.content_rect();
    let mut open = true;
    let sheet = motion::sheet_enter(ctx, Id::NULL.with(id).with("enter"));
    let t = sheet.alpha;
    egui::Area::new(Id::NULL.with(id).with("scrim")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
        let r = ui.allocate_rect(screen, Sense::click());
        ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha((140.0 * t) as u8));
        if r.clicked() { open = false; }
    });
    let rect = Rect::from_center_size(screen.center(), size * sheet.scale);
    egui::Area::new(Id::NULL.with(id)).order(egui::Order::Tooltip).fixed_pos(rect.min).show(ctx, |ui| {
        ui.set_opacity(t);
        ui.set_clip_rect(screen);
        let (_, resp) = ui.allocate_exact_size(size, Sense::click());
        let _ = resp;
        glass_at(ui, rect, 28.0, 1.6);
        text(ui, rect.min + Vec2::new(24.0, 32.0), Align2::LEFT_CENTER, title, 18.0, 1.0);
        ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_max(rect.min + Vec2::new(24.0, 60.0), rect.max - Vec2::splat(24.0))), content);
    });
    open
}
