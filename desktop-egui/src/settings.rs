//! App settings: the language, the glass background, the battery readout.

use crate::glass::{self, Choice, Trailing};
use crate::{icons, load_settings, save_setting, t, translations, chosen_locale, App};
use eframe::egui::{Align2, Pos2, Rect, Ui, Vec2};

const ROW: f32 = 72.0;
const MAX_W: f32 = 760.0;

/// Settings > Language, as on the phone: (tag in `strings::LOCALES`, native name); "" is the system default.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("", ""), ("en", "English"), ("zh-CN", "简体中文"), ("zh-TW", "繁體中文"), ("ja", "日本語"), ("ko", "한국어"),
    ("in", "Bahasa Indonesia"), ("ms", "Bahasa Melayu"), ("cs", "Čeština"), ("de", "Deutsch"), ("es", "Español"),
    ("fil", "Filipino"), ("fr", "Français"), ("it", "Italiano"), ("hu", "Magyar"), ("nl", "Nederlands"),
    ("pl", "Polski"), ("pt", "Português"), ("ro", "Română"), ("sv", "Svenska"), ("vi", "Tiếng Việt"),
    ("tr", "Türkçe"), ("el", "Ελληνικά"), ("ru", "Русский"), ("uk", "Українська"), ("hi", "हिन्दी"),
    ("bn", "বাংলা"), ("th", "ไทย"),
];

#[derive(Default)]
pub struct State { choosing: bool }

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    let tr = app.tr.clone();
    let w = body.width().min(MAX_W);
    let x0 = body.left();
    let mut y = body.top();
    let saved = load_settings()["language"].as_str().unwrap_or("").to_string();
    let current = LANGUAGES.iter().position(|l| l.0 == saved).unwrap_or(0);
    let name = |i: usize| if i == 0 { t(&tr, "language_system").to_string() } else { LANGUAGES[i].1.to_string() };

    let clear = load_settings()["clear_battery"].as_bool() == Some(true);
    let rows = 3.0;
    let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW * rows));
    glass::glass_at(ui, card, 28.0, 1.0);
    let r = |i: f32| Rect::from_min_size(Pos2::new(x0, y + ROW * i), Vec2::new(w, ROW));
    let (clicked, _) = glass::row_at(ui, "lang", r(0.0), icons::LANGUAGE, t(&tr, "settings_language_title"), t(&tr, "settings_language_sub"), glass::Trailing::Value(name(current)), true);
    if clicked { app.settings.choosing = true; }
    glass::divider_at(ui, x0, x0 + w, r(1.0).top());
    let (_, toggled) = glass::row_at(ui, "glow", r(1.0), icons::PALETTE, t(&tr, "theme_glass_gradient"), t(&tr, "theme_glass_gradient_sub"), Trailing::Switch(app.glow), true);
    if toggled { app.glow = !app.glow; save_setting("glass_gradient", app.glow.into()); }
    glass::divider_at(ui, x0, x0 + w, r(2.0).top());
    let (_, toggled) = glass::row_at(ui, "clearbat", r(2.0), icons::BATTERY, t(&tr, "settings_clear_battery_title"), t(&tr, "settings_clear_battery_sub"), Trailing::Switch(clear), true);
    if toggled { save_setting("clear_battery", (!clear).into()); }
    y += card.height() + 24.0;
    glass::text(ui, Pos2::new(x0 + 6.0, y + 8.0), Align2::LEFT_CENTER, &format!("QuickBuds {}", env!("CARGO_PKG_VERSION")), 13.0, 0.6);
    y += 24.0;

    if app.settings.choosing {
        let options: Vec<String> = (0..LANGUAGES.len()).map(name).collect();
        match glass::choice_dialog(ui.ctx(), "lang-dialog", t(&tr, "settings_language_title"), &options, current) {
            Choice::Pick(i) => {
                save_setting("language", LANGUAGES[i].0.into());
                app.tr = translations(chosen_locale());
                app.settings.choosing = false;
            }
            Choice::Close => app.settings.choosing = false,
            Choice::Open => {}
        }
    }
    y
}
