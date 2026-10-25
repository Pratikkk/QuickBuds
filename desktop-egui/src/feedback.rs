//! Feedback (`ReportActivity`, DESIGN §7.16): the model, six categories, the description, the log switch,
//! the JSON preview and Send. The Worker post is not wired on the desktop yet; Send keeps the report.

use crate::glass::{self, white};
use crate::phone;
use crate::settings::ui::section;
use crate::settings::sheets::{self, Title, WIDTH};
use crate::{t, App};
use eframe::egui::{self, Align2, Pos2, Rect, Sense, Ui, Vec2};

const ROW: f32 = 56.0;
const CATS: [&str; 6] = ["problem_cat_ui", "problem_cat_lag", "problem_cat_connection", "problem_cat_feature", "problem_cat_battery", "problem_cat_other"];

#[derive(Default)]
pub struct State {
    pub ready: bool,
    pub model: String,
    cats: [bool; 6],
    desc: String,
    attach: bool,
    pub preview: bool,
    sent: bool,
}

fn payload(app: &App) -> String {
    let tr = &app.tr;
    let cats: Vec<&str> = CATS.iter().enumerate().filter(|(i, _)| app.feedback.cats[*i]).map(|(_, k)| t(tr, k)).collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "kind": "feedback", "model": app.feedback.model, "category": cats, "description": app.feedback.desc,
        "log": if app.feedback.attach { "attached" } else { "none" }, "app": env!("CARGO_PKG_VERSION"),
    })).unwrap_or_default()
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    if !app.feedback.ready { app.feedback.ready = true; app.feedback.model = app.snap.name.clone(); app.feedback.attach = true; }
    let tr = app.tr.clone();
    let w = phone::content(body);
    let x0 = body.left();
    let mut y = body.top() + 0.2;
    let accent = crate::palette::active(false, false).accent;

    phone::paragraph(ui, Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 24.0)), t(&tr, "problem_intro"), 13.0, white(0.74), false);
    y += 22.5;

    section(ui, x0, y, t(&tr, "problem_model"), white(0.70));
    y += phone::SECTION;
    let field = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 50.7));
    phone::field_at(ui, field, &mut app.feedback.model, 40);
    y += 50.7 + 2.0;

    section(ui, x0, y, t(&tr, "problem_category"), white(0.70));
    y += phone::SECTION;
    glass::flat_at(ui, Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW * 6.0 + 8.0 * 5.0)), 24.0, 1.0);
    for (i, key) in CATS.iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(x0, y + (ROW + 8.0) * i as f32), Vec2::new(w, ROW));
        let on = app.feedback.cats[i];
        let hit = ui.interact(r, ui.scope_id().with(("cat", i)), Sense::click());
        glass::text(ui, Pos2::new(r.left() + 32.0, r.center().y), Align2::LEFT_CENTER, t(&tr, key), 16.0, 1.0);
        glass::switch_at(ui, ("cat-sw", i), Pos2::new(r.right() - 16.0 - 52.0, r.center().y - 16.0), on, true);
        if hit.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
        if hit.clicked() { app.feedback.cats[i] = !on; }
    }
    y += ROW * 6.0 + 8.0 * 5.0 + 2.0;

    section(ui, x0, y, t(&tr, "problem_description"), white(0.70));
    y += phone::SECTION;
    let area = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 109.0));
    glass::flat_at(ui, area, 16.0, 1.0);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area.shrink2(Vec2::new(16.0, 12.0))));
    child.add(egui::TextEdit::multiline(&mut app.feedback.desc).frame(egui::Frame::NONE).desired_rows(4)
        .hint_text(egui::RichText::new(t(&tr, "problem_description_hint")).color(white(0.5))).text_color(white(1.0)));
    y += 109.0 + 16.0;

    let log = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 75.4));
    glass::flat_at(ui, log, 16.0, 1.0);
    glass::text(ui, Pos2::new(log.left() + 53.7, log.top() + 27.0), Align2::LEFT_CENTER, t(&tr, "problem_log_title"), 16.0, 1.0);
    glass::text(ui, Pos2::new(log.left() + 53.7, log.top() + 49.0), Align2::LEFT_CENTER, t(&tr, "problem_log_sub"), 13.0, 0.74);
    glass::icon_at(ui, crate::icons::DEV_TOOLS, Rect::from_center_size(Pos2::new(log.left() + 16.0 + 12.0, log.center().y), Vec2::splat(24.0)), white(0.8));
    if glass::switch_at(ui, "log-sw", Pos2::new(log.right() - 16.0 - 52.0, log.center().y - 16.0), app.feedback.attach, true) {
        app.feedback.attach = !app.feedback.attach;
    }
    y += 75.4 + 20.0;

    let known = !app.feedback.model.trim().is_empty();
    let ready = known && !app.feedback.desc.trim().is_empty() && app.feedback.attach;
    let bw = (w - 12.0) / 2.0;
    let see = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(bw, 52.0));
    let send = Rect::from_min_size(Pos2::new(x0 + bw + 12.0, y), Vec2::new(bw, 52.0));
    if phone::pill(ui, "fb-preview", see, t(&tr, "problem_preview"), false, accent) { app.feedback.preview = true; }
    if phone::pill(ui, "fb-send", send, t(&tr, "problem_send"), ready, accent) && ready { app.feedback.sent = true; }
    if app.feedback.sent {
        y += 52.0 + 8.0;
        glass::text(ui, Pos2::new(x0 + 4.0, y), Align2::LEFT_CENTER, t(&tr, "problem_sent"), 13.0, 0.9);
        y += 20.0;
    }

    if app.feedback.preview {
        let body = payload(app);
        let mut decision = None;
        let open = sheets::sheet(ui.ctx(), "fb-preview", t(&tr, "problem_preview"), Title::Centre, Vec2::new(WIDTH, 420.0), |ui, inner| {
            let log = Rect::from_min_max(inner.min + Vec2::new(14.0, 0.0), Pos2::new(inner.right() - 14.0, inner.bottom() - 66.0));
            glass::flat_at(ui, log, 12.0, 1.0);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(log.shrink(10.0)));
            child.set_clip_rect(log.shrink(10.0));
            egui::ScrollArea::vertical().show(&mut child, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(&body).monospace().size(12.0).color(white(0.8))));
            });
            decision = sheets::actions(ui, inner, t(&tr, "dialog_close"), if ready { t(&tr, "problem_send") } else { "" });
        });
        if decision == Some(true) && ready { app.feedback.sent = true; app.feedback.preview = false; }
        else if decision.is_some() || !open { app.feedback.preview = false; }
    }
    y + 24.0
}
