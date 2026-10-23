//! Dev tools: the packet log (Simple / Detailed), Clear, Export, Reconnect, Disconnect. The lines are formatted by
//! `devlog`, shared with the Slint app. Export saves the raw lines.

use crate::devlog::{format, raw, CONNECTION, PROBLEM, RECEIVED};
use crate::glass::{self, white};
use crate::session::{Cmd, LOG};
use crate::App;
use eframe::egui::{Align2, Color32, FontId, Pos2, Rect, Ui, Vec2};
use std::time::Duration;

const MAX_W: f32 = 900.0;
const LINES: usize = 400;

#[derive(Default)]
pub struct State { mode: i32, note: String }

fn colour(kind: i32) -> Color32 {
    match kind {
        RECEIVED => white(0.92),
        CONNECTION => Color32::from_rgb(150, 225, 170),
        PROBLEM => Color32::from_rgb(255, 160, 130),
        _ => Color32::from_rgb(165, 195, 255),
    }
}

fn button(ui: &mut glass::Ui, id: &str, x: &mut f32, y: f32, label: &str, lit: bool) -> bool {
    let w = 36.0 + label.chars().count() as f32 * 8.5;
    let r = Rect::from_min_size(Pos2::new(*x, y), Vec2::new(w, 42.0));
    *x += w + 8.0;
    let hit = glass::button_at(ui, id, r, 21.0, lit).clicked();
    ui.painter().text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional(14.0), if lit { glass::on_accent() } else { glass::white(1.0) });
    hit
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    ui.ctx().request_repaint_after(Duration::from_millis(500));
    let w = body.width().min(MAX_W);
    let x0 = body.left();
    let mut x = x0;
    let y = body.top();
    let mode = app.dev.mode;
    if button(ui, "simple", &mut x, y, "Simple", mode == 0) { app.dev.mode = 0; }
    if button(ui, "detailed", &mut x, y, "Detailed", mode == 1) { app.dev.mode = 1; }
    x += 12.0;
    if button(ui, "export", &mut x, y, "Export", false) {
        let text: String = LOG.lock().unwrap().iter().map(|l| raw(l) + "\n").collect();
        let dir = crate::settings_path().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_else(std::env::temp_dir);
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let path = dir.join(format!("quickbuds-log-{secs}.txt"));
        app.dev.note = match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, text)) {
            Ok(_) => format!("Saved to {}", path.display()),
            Err(e) => format!("Export failed: {e}"),
        };
    }
    if button(ui, "clear", &mut x, y, "Clear", false) { LOG.lock().unwrap().clear(); }
    if button(ui, "reconnect", &mut x, y, "Reconnect", false) { app.send(Cmd::Disconnect); app.send(Cmd::Connect); }
    if button(ui, "disconnect", &mut x, y, "Disconnect", false) { app.send(Cmd::Disconnect); }
    let mut top = y + 54.0;
    if !app.dev.note.is_empty() {
        glass::text(ui, Pos2::new(x0 + 6.0, top + 8.0), Align2::LEFT_CENTER, &app.dev.note, 13.0, 0.75);
        top += 26.0;
    }
    // Newest first.
    let rows: Vec<(String, i32)> = LOG.lock().unwrap().iter().rev().filter_map(|l| format(l, app.dev.mode)).take(LINES).collect();
    let line_h = 20.0;
    let heights: Vec<f32> = rows.iter().map(|(t, _)| line_h * t.lines().count().max(1) as f32).collect();
    let total: f32 = heights.iter().sum::<f32>() + 28.0;
    let card = Rect::from_min_size(Pos2::new(x0, top), Vec2::new(w, total.max(80.0)));
    glass::flat_at(ui, card, 28.0, 1.0);
    let mut cy = top + 14.0;
    for ((text, kind), h) in rows.iter().zip(&heights) {
        // Rows far outside the window are not drawn.
        if cy + h > ui.clip_rect().top() - 40.0 && cy < ui.clip_rect().bottom() + 40.0 {
            ui.painter().text(Pos2::new(x0 + 20.0, cy), Align2::LEFT_TOP, text, FontId::monospace(13.0), colour(*kind));
        }
        cy += h;
    }
    if rows.is_empty() {
        glass::text(ui, card.center(), Align2::CENTER_CENTER, "No lines yet", 14.0, 0.6);
    }
    card.bottom()
}
