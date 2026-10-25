//! Home layout (`HomeLayoutActivity`, DESIGN §7.5): the phone's Home under an "Edit layout" header, with
//! 61.3 dp rows, a 44 dp eye and a live drag. Apply writes `home_rows` and `home_hidden`.

use crate::glass::{self, white};
use crate::overview::home;
use crate::icons;
use crate::phone;
use crate::{load_settings, save_setting, t, App, Page};
use eframe::egui::{self, Align2, Pos2, Rect, Sense, Ui, Vec2};

const HINT: f32 = 46.1;
const ROW: f32 = 61.3;
const GAP: f32 = 8.0;

#[derive(Default)]
pub struct State {
    ready: bool,
    order: Vec<String>,
    hidden: Vec<String>,
    drag: Option<usize>,
}

/// Reads the saved order and hidden set when the screen opens.
pub fn opened(app: &mut App) {
    let rows = home::row_list(app);
    let st = load_settings();
    let saved: Vec<String> = st["home_rows"].as_str().unwrap_or("").split(',').filter(|s| !s.is_empty()).map(String::from).collect();
    let mut order: Vec<String> = Vec::new();
    let mut all: Vec<String> = rows.iter().map(|r| r.id.to_string()).collect();
    for k in saved { if all.contains(&k) && !order.contains(&k) { order.push(k); } }
    for k in all.drain(..) { if !order.contains(&k) { order.push(k); } }
    app.layout.order = order;
    app.layout.hidden = st["home_hidden"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
    app.layout.drag = None;
    app.layout.ready = true;
}

/// Apply: the new order and hidden set go to `settings.json`, then Home.
pub fn apply(app: &mut App) {
    save_setting("home_rows", app.layout.order.join(",").into());
    save_setting("home_hidden", serde_json::json!(app.layout.hidden));
    app.layout.ready = false;
    app.page = Page::Overview;
}

/// Cancel: Home draws the saved order again.
pub fn cancel(app: &mut App) {
    app.layout.ready = false;
    app.page = Page::Overview;
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    if !app.layout.ready { opened(app); }
    let tr = app.tr.clone();
    let w = phone::content(body);
    let x0 = body.left();
    let mut y = body.top() + 16.0;

    let hint_h = phone::wrap_height(ui, t(&tr, "layout_hint"), 13.0, w - 8.0).max(16.0);
    phone::paragraph(ui, Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, hint_h + 4.0)), t(&tr, "layout_hint"), 13.0, white(0.74), false);
    y += HINT;

    let battery = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, home::BATTERY));
    home::battery_at(app, ui, battery);
    y += home::BATTERY + 12.0;
    let noise = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, home::NOISE));
    home::noise_at(app, ui, noise, false);
    y += home::NOISE + 12.0;

    // The order list: every supported row, hidden or not.
    let rows = home::row_list(app);
    let index = |id: &str| rows.iter().position(|r| r.id == id);
    let list: Vec<(String, String, String, &'static str)> = app.layout.order.iter().filter_map(|k| {
        let r = index(k).map(|i| &rows[i])?;
        Some((r.id.to_string(), r.title.clone(), r.sub.clone(), r.icon))
    }).collect();
    let dragging = app.layout.drag;
    let pointer = ui.ctx().input(|i| i.pointer.interact_pos());
    if let (Some(from), Some(p)) = (dragging, pointer) {
        let slot = (((p.y - y) / (ROW + GAP)).round().max(0.0) as usize).min(list.len().saturating_sub(1));
        if slot != from && from < app.layout.order.len() {
            let moved = app.layout.order.remove(from);
            app.layout.order.insert(slot, moved);
            app.layout.drag = Some(slot);
        }
    }
    let mut eye = None;
    for (i, (id, title, sub, _)) in list.iter().enumerate() {
        let rect = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW));
        let is_drag = app.layout.drag == Some(i);
        let hidden = app.layout.hidden.iter().any(|k| k == id);
        let r = ui.interact(rect, ui.scope_id().with(("layout", id)), Sense::click_and_drag());
        if r.drag_started() { app.layout.drag = Some(i); }
        if r.drag_stopped() { app.layout.drag = None; }
        let a = if is_drag { 0.15 } else if hidden { 0.35 } else { 1.0 };
        glass::flat_at(ui, rect, 16.0, if r.hovered() { 1.4 } else { 1.0 });
        let x = rect.left() + 53.7;
        let text_w = rect.width() - 53.7 - 44.0 - 16.0;
        glass::text(ui, Pos2::new(x, rect.top() + 20.0), Align2::LEFT_CENTER, &crate::settings::ui::fit(ui, title, 16.0, text_w), 16.0, a);
        if !sub.is_empty() {
            glass::text(ui, Pos2::new(x, rect.top() + 41.0), Align2::LEFT_CENTER, &crate::settings::ui::fit(ui, sub, 13.0, text_w), 13.0, 0.74 * a);
        }
        let eye_rect = Rect::from_center_size(Pos2::new(rect.right() - 16.0 - 22.0, rect.center().y), Vec2::splat(44.0));
        let er = ui.interact(eye_rect, ui.scope_id().with(("eye", id)), Sense::click());
        glass::icon_at(ui, if hidden { icons::EYE_OFF } else { icons::EYE }, Rect::from_center_size(eye_rect.center(), Vec2::splat(22.0)), white(0.85 * a));
        if er.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
        if er.clicked() { eye = Some(i); }
        y += ROW + GAP;
    }
    if let Some(i) = eye {
        let id = list.get(i).map(|r| r.0.clone());
        if let Some(id) = id {
            if let Some(at) = app.layout.hidden.iter().position(|k| *k == id) { app.layout.hidden.remove(at); }
            else { app.layout.hidden.push(id); }
        }
    }
    if app.layout.drag.is_some() && ui.ctx().input(|i| i.pointer.any_released()) { app.layout.drag = None; }
    y
}
