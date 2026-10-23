//! The model list (`ModelActivity`): Automatic, then every model in HeyMelody's list by brand. A pick
//! overrides detection until Automatic is picked again or buds with another product id connect.

use crate::glass;
use crate::session::{Cmd, Status};
use crate::types::models;
use crate::{load_settings, save_setting, t, App, Page};
use eframe::egui::{Align2, Pos2, Rect, Ui, Vec2};

const ROW: f32 = 56.0;
const MAX_W: f32 = 760.0;

/// What the Earbud settings row says: the pick, or Automatic with what was detected.
pub fn summary(app: &App) -> String {
    let s = &app.snap;
    let detected = s.detected.and_then(|m| m["name"].as_str());
    match s.model.and_then(|m| m["name"].as_str()).filter(|_| s.manual) {
        Some(n) => n.to_string(),
        None => format!("{} · {}", t(&app.tr, "model_auto"), match detected {
            Some(n) => t(&app.tr, "model_detected").replace("%1$s", n),
            None => t(&app.tr, "model_not_detected").to_string(),
        }),
    }
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    let tr = app.tr.clone();
    let s = app.snap.clone();
    let w = body.width().min(MAX_W);
    let x0 = body.left();
    let mut y = body.top();
    // Connected, the session's word counts (buds with another product id reset the pick there).
    let current = if s.status == Status::On {
        if s.manual { s.model.and_then(|m| m["id"].as_str()).unwrap_or("").to_string() } else { String::new() }
    } else { load_settings()["model_manual"].as_str().unwrap_or("").to_string() };

    let back = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 44.0));
    if glass::button_at(ui, "back", Rect::from_min_size(back.min, Vec2::new(120.0, 44.0)), 22.0, false).clicked() { app.page = Page::Earbuds; }
    glass::icon_at(ui, crate::icons::CHEVRON_RIGHT, Rect::from_center_size(Pos2::new(x0 + 26.0, back.center().y), Vec2::splat(18.0)), glass::white(1.0));
    glass::text(ui, Pos2::new(x0 + 48.0, back.center().y), Align2::LEFT_CENTER, t(&tr, "earbuds_title"), 14.0, 1.0);
    y += 62.0;

    let mut pick: Option<String> = None;
    let all = models();
    let name = |m: &serde_json::Value| m["name"].as_str().unwrap_or("").to_string();

    let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW));
    glass::flat_at(ui, card, 28.0, 1.0);
    let detected = s.detected.and_then(|m| m["name"].as_str());
    let auto_sub = match detected {
        Some(n) => t(&tr, "model_detected").replace("%1$s", n),
        None => t(&tr, "model_not_detected").to_string(),
    };
    if glass::list_item_at(ui, "auto", card, &format!("{}   ·   {}", t(&tr, "model_auto"), auto_sub), current.is_empty(), false).clicked() { pick = Some(String::new()); }
    y += ROW + 18.0;

    for brand in ["OnePlus", "OPPO", "realme", "DIZO"] {
        let mut list: Vec<&serde_json::Value> = all.iter().filter(|m| name(m).starts_with(&format!("{brand} "))).collect();
        if list.is_empty() { continue; }
        list.sort_by_key(|m| name(m).to_lowercase());
        glass::text(ui, Pos2::new(x0 + 6.0, y + 14.0), Align2::LEFT_CENTER, brand, 14.0, 0.72);
        y += 34.0;
        let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW * list.len() as f32));
        glass::flat_at(ui, card, 28.0, 1.0);
        for (i, m) in list.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(x0, y + ROW * i as f32), Vec2::new(w, ROW));
            if i > 0 { glass::divider_at(ui, x0, x0 + w, r.top()); }
            let n = name(m);
            let id = m["id"].as_str().unwrap_or("");
            // Two ids share a name (colour ranges, regional variants): those rows show their id.
            let shared = all.iter().filter(|x| name(x) == n).count() > 1;
            let label = if shared { format!("{n}   ·   {id}") } else { n };
            if glass::list_item_at(ui, ("model", id.to_string()), r, &label, current == id, false).clicked() { pick = Some(id.to_string()); }
        }
        y += card.height() + 18.0;
    }

    if let Some(id) = pick {
        save_setting("model_manual", if id.is_empty() { serde_json::Value::Null } else { id.as_str().into() });
        app.send(Cmd::ModelPicked);
    }
    y
}
