//! Language (`LanguageActivity`, DESIGN §7.8): System default and the 27 languages of the phone's `values-*` folders.
//! A tap applies at once and saves the tag in `settings.json` "language".

use crate::glass;
use crate::phone;
use crate::settings::LANGUAGES;
use crate::{load_settings, save_setting, t, translations, App};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Ui, Vec2};

const ROW: f32 = 56.0;
const GAP: f32 = 2.0;

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    let saved = load_settings()["language"].as_str().unwrap_or("").to_string();
    let current = LANGUAGES.iter().position(|l| l.0 == saved).unwrap_or(0);
    let w = phone::content(body);
    let x0 = body.left();
    let accent = crate::palette::active(crate::palette::is_light_system(ui.ctx()), false).accent;
    let mut y = body.top();
    let mut pick: Option<usize> = None;
    for (i, lang) in LANGUAGES.iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW));
        glass::flat_at(ui, r, phone::RADIUS, 1.0);
        if i == current { ui.painter().rect_filled(r, phone::RADIUS, glass::sel_fill(0.85)); }
        let hit = ui.interact(r, ui.scope_id().with(("lang", i)), Sense::click());
        if hit.hovered() && i != current { ui.painter().rect_filled(r.shrink(6.0), 20.0, glass::white(0.06)); }
        let label = if i == 0 { t(&app.tr, "language_system") } else { lang.1 };
        // The chosen row's label takes the accent (the phone's rule for a selected list row).
        let colour = if i == current { accent } else { glass::white(1.0) };
        ui.painter().text(Pos2::new(x0 + phone::MARGIN, r.center().y), Align2::LEFT_CENTER, label, FontId::proportional(16.0), colour);
        if hit.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
        if hit.clicked() { pick = Some(i); }
        y += ROW + GAP;
    }
    if let Some(i) = pick {
        save_setting("language", LANGUAGES[i].0.into());
        app.tr = translations(crate::chosen_locale());
    }
    y - GAP + phone::GAP
}
