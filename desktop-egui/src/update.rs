//! App update (`UpdateActivity`, DESIGN §7.18): installed and latest, the status line, the action, the
//! check-on-start row and the notes. The GitHub check and the download are not wired on the desktop yet.

use crate::glass::{self, white};
use crate::phone;
use crate::settings::ui::section;
use crate::{load_settings, save_setting, t, App};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Ui, Vec2};

#[derive(Default)]
pub struct State {
    checked: bool,
    latest: String,
    checking: bool,
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    let tr = app.tr.clone();
    let w = phone::content(body);
    let x0 = body.left();
    let mut y = body.top() + 12.0;
    let installed = env!("CARGO_PKG_VERSION");
    if app.update.latest.is_empty() { app.update.latest = installed.into(); }
    let accent = crate::palette::active(false, false).accent;

    // Installed | Latest.
    let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 86.1));
    glass::flat_at(ui, card, 24.0, 1.0);
    for (i, (value, key)) in [(installed, "update_installed"), (app.update.latest.as_str(), "update_latest")].iter().enumerate() {
        let cx = card.left() + card.width() * (0.25 + 0.5 * i as f32);
        ui.painter().text(Pos2::new(cx, card.top() + 34.0), Align2::CENTER_CENTER, *value, FontId::proportional(22.0), white(1.0));
        ui.painter().text(Pos2::new(cx, card.top() + 62.0), Align2::CENTER_CENTER, t(&tr, key), FontId::proportional(13.0), white(0.74));
    }
    y += 86.1;

    // The status line.
    let status = if app.update.checking { t(&tr, "update_checking") } else if app.update.checked { t(&tr, "update_up_to_date") } else { t(&tr, "update_up_to_date") };
    ui.painter().text(Pos2::new(x0 + w / 2.0, y + 17.0), Align2::CENTER_CENTER, status, FontId::proportional(14.0), white(0.85));
    y += 33.9 + 10.0;

    let action = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 44.0));
    let label = if app.update.checking { t(&tr, "update_checking") } else { t(&tr, "update_check") };
    if phone::pill(ui, "update-check", action, label, true, accent) {
        app.update.checking = true;
        app.update.checked = true;
        app.update.latest = installed.into();
    }
    if app.update.checking { app.update.checking = false; }
    y += 44.0 + 20.0;

    let auto = load_settings()["update_auto"].as_bool().unwrap_or(true);
    let row = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 75.4));
    glass::flat_at(ui, row, 16.0, 1.0);
    let hit = ui.interact(row, ui.scope_id().with("update-auto"), egui::Sense::click());
    glass::text(ui, Pos2::new(row.left() + 16.0, row.top() + 29.0), Align2::LEFT_CENTER, t(&tr, "update_auto_title"), 16.0, 1.0);
    glass::text(ui, Pos2::new(row.left() + 16.0, row.top() + 51.0), Align2::LEFT_CENTER, t(&tr, "update_auto_sub"), 13.0, 0.74);
    let mut toggled = glass::switch_at(ui, "update-auto-sw", Pos2::new(row.right() - 16.0 - 52.0, row.center().y - 16.0), auto, true);
    if hit.clicked() { toggled = true; }
    if toggled { save_setting("update_auto", (!auto).into()); }
    if hit.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
    y += 75.4 + 16.0;

    section(ui, x0, y, t(&tr, "update_notes"), white(0.70));
    y += phone::SECTION;
    let notes = t(&tr, "update_no_notes");
    let h = (phone::wrap_height(ui, notes, 14.0, w - 32.0) + 28.0).max(80.0);
    let box_rect = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, h));
    glass::flat_at(ui, box_rect, 24.0, 1.0);
    phone::paragraph(ui, box_rect.shrink2(Vec2::new(16.0, 14.0)), notes, 14.0, white(0.74), false);
    y += h;
    y + 20.0
}
