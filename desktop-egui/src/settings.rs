//! App settings: the language, the glass background, the battery readout.

use crate::glass::{self, style::Style, Choice, Trailing};
use crate::palette;
use crate::{icons, load_settings, save_setting, t, translations, chosen_locale, App};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Ui, Vec2};

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
pub struct State { choosing: bool, style_dlg: bool, theme_dlg: bool, accent_dlg: bool }

const ACCENTS: [&str; 12] = ["#D71920", "#E8590C", "#F59F00", "#40C057", "#12B886", "#15AABF", "#339AF0", "#5C7CFA", "#845EF7", "#CC5DE8", "#F06595", "#C2C6D6"];

fn theme_name(tr: &[String], id: &str) -> String {
    t(tr, match id { palette::OLED => "theme_oled_black", palette::DARK => "theme_classic_dark", palette::WHITE => "theme_white", _ => "theme_custom" }).to_string()
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    let tr = app.tr.clone();
    let w = body.width().min(MAX_W);
    let x0 = body.left();
    let mut y = body.top();
    let saved = load_settings()["language"].as_str().unwrap_or("").to_string();
    let current = LANGUAGES.iter().position(|l| l.0 == saved).unwrap_or(0);
    let name = |i: usize| if i == 0 { t(&tr, "language_system").to_string() } else { LANGUAGES[i].1.to_string() };

    let clear = load_settings()["clear_battery"].as_bool() == Some(true);
    let is_glass = app.style == Style::Glass;
    // Rows: the style first; the theme and accent for the styles that use the palette; the glow for Glass.
    let rows = 3 + if is_glass { 1 } else { 2 };
    let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW * rows as f32));
    glass::glass_at(ui, card, 28.0, 1.0);
    let r = |i: usize| Rect::from_min_size(Pos2::new(x0, y + ROW * i as f32), Vec2::new(w, ROW));
    let mut i = 0;
    let (clicked, _) = glass::row_at(ui, "style", r(i), icons::PALETTE, t(&tr, "widget_style_title"), "", Trailing::Value(t(&tr, app.style.name_key()).into()), true);
    if clicked { app.settings.style_dlg = true; }
    i += 1;
    if is_glass {
        glass::divider_at(ui, x0, x0 + w, r(i).top());
        let (_, toggled) = glass::row_at(ui, "glow", r(i), icons::PALETTE, t(&tr, "theme_glass_gradient"), t(&tr, "theme_glass_gradient_sub"), Trailing::Switch(app.glow), true);
        if toggled { app.glow = !app.glow; save_setting("glass_gradient", app.glow.into()); }
        i += 1;
    } else {
        let light = palette::is_light_system(ui.ctx());
        let id = palette::active_id(light);
        let name = if palette::auto() { t(&tr, "theme_auto").to_string() } else { theme_name(&tr, &id) };
        glass::divider_at(ui, x0, x0 + w, r(i).top());
        let (clicked, _) = glass::row_at(ui, "theme", r(i), icons::PALETTE, t(&tr, "theme_title"), "", Trailing::Value(name), true);
        if clicked { app.settings.theme_dlg = true; }
        i += 1;
        glass::divider_at(ui, x0, x0 + w, r(i).top());
        let (clicked, _) = glass::row_at(ui, "accent", r(i), icons::PALETTE, t(&tr, "theme_accent"), "", Trailing::Value(palette::hex(crate::glass::style::pal().accent)), true);
        if clicked { app.settings.accent_dlg = true; }
        i += 1;
    }
    glass::divider_at(ui, x0, x0 + w, r(i).top());
    let (clicked, _) = glass::row_at(ui, "lang", r(i), icons::LANGUAGE, t(&tr, "settings_language_title"), t(&tr, "settings_language_sub"), glass::Trailing::Value(name(current)), true);
    if clicked { app.settings.choosing = true; }
    i += 1;
    glass::divider_at(ui, x0, x0 + w, r(i).top());
    let (_, toggled) = glass::row_at(ui, "clearbat", r(i), icons::BATTERY, t(&tr, "settings_clear_battery_title"), t(&tr, "settings_clear_battery_sub"), Trailing::Switch(clear), true);
    if toggled { save_setting("clear_battery", (!clear).into()); }
    y += card.height() + 24.0;
    glass::text(ui, Pos2::new(x0 + 6.0, y + 8.0), Align2::LEFT_CENTER, &format!("QuickBuds {}", env!("CARGO_PKG_VERSION")), 13.0, 0.74);
    y += 24.0;

    // Style
    if app.settings.style_dlg {
        let options: Vec<String> = Style::ALL.iter().map(|s| t(&tr, s.name_key()).to_string()).collect();
        let cur = Style::ALL.iter().position(|s| *s == app.style).unwrap_or(0);
        match glass::choice_dialog(ui.ctx(), "style-dialog", t(&tr, "widget_style_title"), &options, cur) {
            Choice::Pick(i) => {
                save_setting("egui_style", Style::ALL[i].key().into());
                let ctx = ui.ctx().clone();
                app.reload_style(&ctx);
                app.settings.style_dlg = false;
            }
            Choice::Close => app.settings.style_dlg = false,
            Choice::Open => {}
        }
    }
    // Theme: Match system, the three built-in presets, the custom ones.
    if app.settings.theme_dlg {
        let light = palette::is_light_system(ui.ctx());
        let custom = palette::custom();
        let mut ids: Vec<String> = vec!["auto".into(), palette::OLED.into(), palette::DARK.into(), palette::WHITE.into()];
        ids.extend(custom.iter().map(|c| c.0.clone()));
        let mut options: Vec<String> = vec![t(&tr, "theme_auto").into(), theme_name(&tr, palette::OLED), theme_name(&tr, palette::DARK), theme_name(&tr, palette::WHITE)];
        options.extend(custom.iter().map(|c| c.1.clone()));
        let cur = if palette::auto() { 0 } else { ids.iter().position(|i| *i == palette::active_id(light)).unwrap_or(1) };
        match glass::choice_dialog(ui.ctx(), "theme-dialog", t(&tr, "theme_title"), &options, cur) {
            Choice::Pick(i) => {
                if i == 0 { palette::set_auto(true); } else { palette::set_active(&ids[i]); }
                let ctx = ui.ctx().clone();
                app.reload_style(&ctx);
                app.settings.theme_dlg = false;
            }
            Choice::Close => app.settings.theme_dlg = false,
            Choice::Open => {}
        }
    }
    // Accent: swatches for the active built-in preset.
    if app.settings.accent_dlg {
        let light = palette::is_light_system(ui.ctx());
        let id = palette::active_id(light);
        let mut pick: Option<Option<Color32>> = None;
        let open = glass::dialog(ui.ctx(), "accent-dialog", t(&tr, "theme_accent"), Vec2::new(420.0, 250.0), |ui| {
            let r = ui.max_rect();
            for (n, hex) in ACCENTS.iter().enumerate() {
                let c = palette::parse_hex(hex).unwrap();
                let (col, row) = (n % 6, n / 6);
                let center = Pos2::new(r.left() + 28.0 + col as f32 * 58.0, r.top() + 28.0 + row as f32 * 58.0);
                let hit = ui.interact(Rect::from_center_size(center, Vec2::splat(46.0)), ui.scope_id().with(("swatch", n)), Sense::click());
                ui.painter().circle_filled(center, 20.0 + if hit.hovered() { 2.0 } else { 0.0 }, c);
                if crate::glass::style::pal().accent == c { ui.painter().circle_stroke(center, 24.0, egui::Stroke::new(2.0, glass::white(1.0))); }
                if hit.clicked() { pick = Some(Some(c)); }
            }
        });
        match pick {
            Some(c) if palette::BUILT_IN.contains(&id.as_str()) => {
                palette::set_accent(&id, c);
                let ctx = ui.ctx().clone();
                app.reload_style(&ctx);
                app.settings.accent_dlg = false;
            }
            Some(_) => app.settings.accent_dlg = false,
            None => if !open { app.settings.accent_dlg = false },
        }
    }

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
