//! Test my earbuds (`ModelTestActivity`, DESIGN §7.15): Start, the automatic check, a mark per feature,
//! the diagnosis, the JSON preview and Send. The live handshake and the Worker post are not wired on the
//! desktop yet: Start reconnects and the marks are kept for the preview.

use crate::glass::{self, white};
use crate::overview::home;
use crate::phone;
use crate::session::Cmd;
use crate::settings::ui::section;
use crate::settings::sheets::{self, Title, WIDTH};
use crate::{t, App};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Ui, Vec2};

const MAX_W: f32 = 379.4;

#[derive(Default)]
pub struct State {
    pub started: bool,
    answered: [bool; 3],
    /// 0 = not marked, 1 = works, 2 = broken, 3 = skip.
    marks: std::collections::HashMap<String, u8>,
    marking: Option<String>,
    note: String,
    pub preview: bool,
    sent: bool,
}

fn mark_label(tr: &[String], m: u8) -> &str {
    match m { 1 => t(tr, "test_works"), 2 => t(tr, "test_broken"), 3 => t(tr, "test_skipped"), _ => t(tr, "test_not_marked") }
}

fn json(app: &App) -> String {
    let marks: Vec<serde_json::Value> = home::row_list(app).iter().filter_map(|r| {
        let m = *app.test.marks.get(r.id)?;
        Some(serde_json::json!({"feature": r.id, "result": if m == 1 { "works" } else if m == 2 { "broken" } else { "skipped" }}))
    }).collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "kind": "test", "model": app.snap.name, "app": env!("CARGO_PKG_VERSION"),
        "features": marks, "note": app.test.note,
    })).unwrap_or_default()
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    let tr = app.tr.clone();
    let w = phone::content(body).min(MAX_W);
    let x0 = body.left();
    let mut y = body.top() + 0.2;

    phone::paragraph(ui, Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 44.0)), t(&tr, "test_intro"), 13.0, white(0.74), false);
    y += 42.3 + 12.0;
    let start = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 52.0));
    let label = if app.test.started { t(&tr, "test_restart") } else { t(&tr, "test_start") };
    if phone::pill(ui, "test-start", start, label, false, crate::palette::active(false, false).accent) {
        app.send(Cmd::Disconnect);
        app.send(Cmd::Connect);
        app.test.started = true;
        app.test.answered = [app.demo; 3];
        app.test.marks.clear();
        app.test.sent = false;
    }
    y += 52.0 + 16.0;
    if app.test.started && !app.test.answered[0] {
        glass::text(ui, Pos2::new(x0 + 4.0, y + 8.0), Align2::LEFT_CENTER, t(&tr, "test_reconnecting"), 13.0, 0.74);
        y += 26.0;
    }
    if app.test.sent {
        glass::text(ui, Pos2::new(x0 + 4.0, y + 8.0), Align2::LEFT_CENTER, t(&tr, "problem_sent"), 13.0, 0.9);
        y += 26.0;
    }

    if app.test.started {
        section(ui, x0, y, t(&tr, "test_section_check"), white(0.70));
        y += phone::SECTION;
        for (i, key) in ["test_check_battery", "test_check_features", "test_check_states"].iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 56.0));
            glass::flat_at(ui, r, 16.0, 1.0);
            glass::text(ui, Pos2::new(r.left() + 16.0, r.center().y), Align2::LEFT_CENTER, t(&tr, key), 16.0, 1.0);
            let (ok, text) = (app.test.answered[i], t(&tr, if app.test.answered[i] { "test_answered" } else { "test_no_answer" }));
            ui.painter().text(Pos2::new(r.right() - 16.0, r.center().y), Align2::RIGHT_CENTER, text, FontId::proportional(13.0), if ok { white(0.9) } else { white(0.6) });
            y += 56.0 + 8.0;
        }
        y += 12.0;

        section(ui, x0, y, t(&tr, "test_section_features"), white(0.70));
        y += phone::SECTION;
        let rows = home::row_list(app);
        for r in &rows {
            let rect = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 59.8));
            glass::flat_at(ui, rect, 16.0, 1.0);
            glass::icon_at(ui, r.icon, Rect::from_center_size(Pos2::new(rect.left() + 16.0 + 12.0, rect.center().y), Vec2::splat(24.0)), white(0.8));
            // The title starts after the icon; the mark sits at the end.
            glass::text(ui, Pos2::new(rect.left() + 53.7, rect.top() + 19.6), Align2::LEFT_CENTER, &crate::settings::ui::fit(ui, &r.title, 16.0, w - 53.7 - 110.0), 16.0, 1.0);
            let m = *app.test.marks.get(r.id).unwrap_or(&0);
            let hit = ui.interact(rect, ui.scope_id().with(("mark", r.id)), Sense::click());
            ui.painter().text(Pos2::new(rect.right() - 16.0, rect.center().y), Align2::RIGHT_CENTER, mark_label(&tr, m), FontId::proportional(13.0), if m == 1 { white(1.0) } else { white(0.74) });
            if hit.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
            if hit.clicked() { app.test.marking = Some(r.id.to_string()); }
            y += 59.8 + 8.0;
        }
        y += 4.0;
    }

    let all = !home::row_list(app).is_empty() && home::row_list(app).iter().all(|r| app.test.marks.contains_key(r.id));
    let bw = (w - 12.0) / 2.0;
    if !app.test.started { return y + 24.0; }
    let see = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(bw, 52.0));
    let send = Rect::from_min_size(Pos2::new(x0 + bw + 12.0, y), Vec2::new(bw, 52.0));
    let accent = crate::palette::active(false, false).accent;
    if phone::pill(ui, "test-preview", see, t(&tr, "problem_preview"), false, accent) { app.test.preview = true; }
    if phone::pill(ui, "test-send", send, t(&tr, "problem_send"), all, accent) && all { app.test.sent = true; }
    y += 52.0 + 12.0;
    glass::text(ui, Pos2::new(x0 + 4.0, y + 6.0), Align2::LEFT_CENTER, t(&tr, if all { "test_broken_hint" } else { "test_mark_all" }), 12.0, 0.6);

    if let Some(id) = app.test.marking.clone() {
        let options = [t(&tr, "test_works"), t(&tr, "test_broken"), t(&tr, "test_skip")];
        let cur = *app.test.marks.get(&id).unwrap_or(&0);
        let mut decision = None;
        let open = sheets::sheet(ui.ctx(), "mark-sheet", t(&tr, "test_title"), Title::Centre, Vec2::new(WIDTH, 330.0), |ui, inner| {
            for (i, label) in options.iter().enumerate() {
                let r = Rect::from_min_size(Pos2::new(inner.left() + 4.0, inner.top() + 8.0 + 56.0 * i as f32), Vec2::new(inner.width() - 8.0, 56.0));
                if glass::list_item_at(ui, ("mark", i), r, label, cur as usize == i + 1, false).clicked() { decision = Some(i as u8 + 1); }
            }
            let note = Rect::from_min_max(Pos2::new(inner.left() + 14.0, inner.top() + 8.0 + 56.0 * 3.0), Pos2::new(inner.right() - 14.0, inner.bottom() - 66.0));
            phone::paragraph(ui, note, t(&tr, "test_broken_hint"), 12.0, white(0.6), false);
            if sheets::actions(ui, inner, t(&tr, "dialog_cancel"), t(&tr, "test_works")) == Some(true) { decision = Some(1); }
        });
        if let Some(m) = decision { app.test.marks.insert(id, m); app.test.marking = None; }
        else if !open { app.test.marking = None; }
    }

    if app.test.preview {
        let body = json(app);
        let all = home::row_list(app).iter().all(|r| app.test.marks.contains_key(r.id));
        let mut decision = None;
        let open = sheets::sheet(ui.ctx(), "test-preview", t(&tr, "problem_preview"), Title::Centre, Vec2::new(WIDTH, 420.0), |ui, inner| {
            let log = Rect::from_min_max(inner.min + Vec2::new(14.0, 0.0), Pos2::new(inner.right() - 14.0, inner.bottom() - 66.0));
            glass::flat_at(ui, log, 12.0, 1.0);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(log.shrink(10.0)));
            child.set_clip_rect(log.shrink(10.0));
            egui::ScrollArea::vertical().show(&mut child, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(&body).monospace().size(12.0).color(white(0.8))));
            });
            decision = sheets::actions(ui, inner, t(&tr, "dialog_close"), if all { t(&tr, "problem_send") } else { "" });
        });
        if decision == Some(true) && all { app.test.sent = true; app.test.preview = false; }
        else if decision.is_some() || !open { app.test.preview = false; }
    }

    y + 24.0
}
