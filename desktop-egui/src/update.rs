//! App update (`UpdateActivity`, DESIGN §7.18): installed and latest, the status line, the action, the
//! check-on-start row and the notes. The check and the replace are the ones of `desktop/src/update.rs`,
//! through `update_core`; the download must match the release's `SHA256SUMS`.

use crate::glass::{self, white};
use crate::phone;
use crate::settings::ui::section;
use crate::update_core::Release;
use crate::{load_settings, save_setting, t, App};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Ui, Vec2};
use std::sync::mpsc::{channel, Receiver};

#[derive(Default)]
pub struct State {
    opened: bool,
    checking: bool,
    installing: bool,
    /// The newest tag seen; "" until a check answers.
    latest: String,
    status: String,
    found: Option<Release>,
    check: Option<Receiver<Result<Release, String>>>,
    install: Option<Receiver<Result<(), String>>>,
}

fn check(app: &mut App, ctx: &egui::Context) {
    app.update.checking = true;
    app.update.status = t(&app.tr, "update_checking").to_string();
    let (tx, rx) = channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(crate::update_core::latest());
        ctx.request_repaint();
    });
    app.update.check = Some(rx);
}

/// Starts the download and the replace, then quits so the new binary runs alone.
fn install(app: &mut App, ctx: &egui::Context) {
    let Some(r) = app.update.found.take() else { return };
    if !crate::update_core::can_install() {
        crate::update_core::open_page();
        app.update.found = Some(r);
        return;
    }
    app.update.installing = true;
    let (tx, rx) = channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(crate::update_core::install(&r));
        ctx.request_repaint();
    });
    app.update.install = Some(rx);
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    let ctx = ui.ctx().clone();
    if !app.update.opened {
        app.update.opened = true;
        crate::update_core::clean_up();
        check(app, &ctx);
    }
    if let Some(res) = app.update.check.as_ref().and_then(|r| r.try_recv().ok()) {
        app.update.check = None;
        app.update.checking = false;
        match res {
            Ok(r) if crate::update_core::newer(&r.version, env!("CARGO_PKG_VERSION")) => {
                app.update.latest = r.version.clone();
                app.update.status = t(&app.tr, "update_available").replace("%1$s", &r.version);
                app.update.found = Some(r);
            }
            Ok(r) => {
                app.update.latest = r.version;
                app.update.status = t(&app.tr, "update_up_to_date").to_string();
            }
            Err(e) => app.update.status = t(&app.tr, "update_failed").replace("%1$s", &e),
        }
    }
    if let Some(res) = app.update.install.as_ref().and_then(|r| r.try_recv().ok()) {
        app.update.install = None;
        app.update.installing = false;
        match res {
            Ok(()) => { ctx.send_viewport_cmd(egui::ViewportCommand::Close); return 0.0; }
            Err(e) => app.update.status = t(&app.tr, "update_failed").replace("%1$s", &e),
        }
    }
    if app.update.checking || app.update.installing { ctx.request_repaint(); }

    let tr = app.tr.clone();
    let w = phone::content(body);
    let x0 = body.left();
    let mut y = body.top() + 12.0;
    let installed = env!("CARGO_PKG_VERSION");
    let latest = if app.update.latest.is_empty() { t(&tr, "update_unknown") } else { app.update.latest.as_str() };
    let accent = crate::palette::active(false, false).accent;

    // Installed | Latest.
    let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 86.1));
    glass::flat_at(ui, card, 24.0, 1.0);
    for (i, (value, key)) in [(installed, "update_installed"), (latest, "update_latest")].iter().enumerate() {
        let cx = card.left() + card.width() * (0.25 + 0.5 * i as f32);
        ui.painter().text(Pos2::new(cx, card.top() + 34.0), Align2::CENTER_CENTER, *value, FontId::proportional(22.0), white(1.0));
        ui.painter().text(Pos2::new(cx, card.top() + 62.0), Align2::CENTER_CENTER, t(&tr, key), FontId::proportional(13.0), white(0.74));
    }
    y += 86.1;

    // The status line.
    ui.painter().text(Pos2::new(x0 + w / 2.0, y + 17.0), Align2::CENTER_CENTER, &app.update.status, FontId::proportional(14.0), white(0.85));
    y += 33.9 + 10.0;

    let action = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 44.0));
    let (label, ready) = if app.update.installing {
        (t(&tr, "update_download"), false)
    } else if app.update.checking {
        (t(&tr, "update_checking"), false)
    } else if app.update.found.is_some() {
        (t(&tr, "update_install"), true)
    } else {
        (t(&tr, "update_check"), true)
    };
    if phone::pill(ui, "update-check", action, label, ready, accent) && ready {
        if app.update.found.is_some() { install(app, &ctx); } else { check(app, &ctx); }
    }
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
