//! The colours of the non-glass styles: the phone's presets (`ui/Palette.kt`), read from its `values/themes.xml`:
//! OLED Black, Classic Dark and White, an accent per built-in, up to three custom presets, and Match system.
//! The settings are the Slint app's (`palette_*` in `settings.json`), so both apps share one theme.

use crate::{load_settings, save_setting};
use eframe::egui::Color32;
use serde_json::{json, Value};

const THEMES: &str = include_str!("../../app/src/main/res/values/themes.xml");
pub const OLED: &str = "oled";
pub const DARK: &str = "dark";
pub const WHITE: &str = "white";
pub const BUILT_IN: [&str; 3] = [OLED, DARK, WHITE];
/// The phone's accent with the wallpaper colours on, for the Material style.
const PHONE_ACCENT: &str = "#C2C6D6";

#[derive(Clone, Copy, PartialEq)]
pub struct Pal { pub bg: Color32, pub card: Color32, pub accent: Color32, pub text: Color32, pub text2: Color32, pub outline: Color32 }

impl Pal {
    pub const DEFAULT: Pal = Pal {
        bg: Color32::from_rgb(0x12, 0x12, 0x14), card: Color32::from_rgb(0x23, 0x23, 0x27), accent: Color32::from_rgb(0xD7, 0x19, 0x20),
        text: Color32::WHITE, text2: Color32::from_rgb(0xA0, 0xA0, 0xA5), outline: Color32::from_rgb(0x34, 0x34, 0x3A),
    };

    pub fn light(&self) -> bool { luminance(self.bg) > 0.5 }

    /// Material: a card's tonal surface, the card tinted by the accent (a black card is lifted toward the text first).
    pub fn container(&self) -> Color32 {
        let lift = if !self.light() && luminance(self.card) < 0.005 { 0.09 } else { 0.0 };
        blend(blend(self.card, self.text, lift), self.accent, if self.light() { 0.05 } else { 0.08 })
    }

    /// Material: a selected element's fill; its label is `text`.
    pub fn tonal(&self) -> Color32 { blend(self.container(), self.accent, if self.light() { 0.18 } else { 0.28 }) }

    /// The label on an accent fill: the lighter of text and background when it reaches 3:1, else the one that reads better.
    pub fn on_accent(&self) -> Color32 {
        let (light, dark) = if luminance(self.text) > luminance(self.bg) { (self.text, self.bg) } else { (self.bg, self.text) };
        if contrast(light, self.accent) >= 3.0 || contrast(light, self.accent) >= contrast(dark, self.accent) { light } else { dark }
    }

    /// Switch track: the outline, moved toward the text (more on a light theme, where the outline is too faint).
    pub fn track(&self) -> Color32 { blend(self.outline, self.text, if self.light() { 0.14 } else { 0.06 }) }
}

fn luminance(c: Color32) -> f64 {
    let ch = |v: u8| { let s = v as f64 / 255.0; if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) } };
    0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
}

fn contrast(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

pub fn blend(from: Color32, to: Color32, t: f32) -> Color32 {
    let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Color32::from_rgb(m(from.r(), to.r()), m(from.g(), to.g()), m(from.b(), to.b()))
}

pub fn parse_hex(s: &str) -> Option<Color32> {
    let t = s.trim().trim_start_matches('#');
    if t.len() != 6 { return None; }
    let v = u32::from_str_radix(t, 16).ok()?;
    Some(Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

pub fn hex(c: Color32) -> String { format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b()) }

fn xml_color(from: usize, name: &str) -> Color32 {
    let key = format!("name=\"{name}\">");
    let at = from + THEMES[from..].find(&key).expect(name) + key.len();
    parse_hex(&THEMES[at..at + 7]).expect(name)
}

/// A built-in preset from its style in themes.xml, with the user's accent if one is set. `material`: the phone's own
/// neutral colours and pale accent for the dark presets.
pub fn built_in(id: &str, material: bool) -> Pal {
    let style = match id { OLED => "Theme.App.OLED", DARK => "Theme.App.Dark", _ => "Theme.App.Light" };
    let at = THEMES.find(&format!("<style name=\"{style}\"")).expect(style);
    let c = |n: &'static str| xml_color(at, n);
    let phone = material && id != WHITE;
    let accent = load_settings()["palette_accent"][id].as_str().and_then(parse_hex).or(if phone { parse_hex(PHONE_ACCENT) } else { None });
    let tok = |name: &'static str, phone_hex: &str| if phone { parse_hex(phone_hex).unwrap() } else { c(name) };
    let dark = id == DARK;
    Pal {
        bg: tok("appColorBg", if dark { "#1B1B1D" } else { "#000000" }),
        card: tok("appColorCard", if dark { "#303032" } else { "#000000" }),
        accent: accent.unwrap_or(c("appColorAccent")),
        text: tok("appColorTextPrimary", "#F3F0F1"), text2: tok("appColorTextSecondary", "#C8C6C7"),
        outline: tok("appColorOutline", if dark { "#46474A" } else { "#2E2E30" }),
    }
}

/// (id, name) of the custom presets (`palette_custom`).
pub fn custom() -> Vec<(String, String, Pal)> {
    load_settings()["palette_custom"].as_array().map(|a| a.iter().filter_map(|o| {
        let c = |k: &str| o[k].as_str().and_then(parse_hex);
        Some((o["id"].as_str()?.to_string(), o["name"].as_str()?.to_string(), Pal {
            bg: c("background")?, card: c("card")?, accent: c("accent")?, text: c("text")?, text2: c("textSecondary")?, outline: c("outline")?,
        }))
    }).collect()).unwrap_or_default()
}

pub fn auto() -> bool { load_settings()["palette_auto"].as_bool().unwrap_or(true) }

fn auto_dark() -> &'static str { if load_settings()["palette_auto_dark"] == DARK { DARK } else { OLED } }

pub fn active_id(system_light: bool) -> String {
    if auto() { return if system_light { WHITE } else { auto_dark() }.into(); }
    load_settings()["palette_active"].as_str().unwrap_or(DARK).into()
}

/// The active preset; a deleted custom one falls back to OLED Black.
pub fn active(system_light: bool, material: bool) -> Pal {
    let id = active_id(system_light);
    if BUILT_IN.contains(&id.as_str()) { return built_in(&id, material); }
    custom().into_iter().find(|p| p.0 == id).map(|p| p.2).unwrap_or_else(|| built_in(OLED, material))
}

/// A pick. Under Match system a dark built-in becomes its dark half; any other pick turns Match system off.
pub fn set_active(id: &str) {
    if auto() && (id == OLED || id == DARK) {
        save_setting("palette_auto_dark", id.into());
    } else {
        save_setting("palette_auto", false.into());
        save_setting("palette_active", id.into());
    }
}

pub fn set_auto(on: bool) { save_setting("palette_auto", on.into()); }

/// Sets (or clears, `None`) the accent of a built-in preset.
pub fn set_accent(id: &str, c: Option<Color32>) {
    let mut all = load_settings()["palette_accent"].clone();
    if !all.is_object() { all = json!({}); }
    match c { Some(c) => all[id] = hex(c).into(), None => { all.as_object_mut().map(|o| o.remove(id)); } }
    save_setting("palette_accent", all);
}

pub fn accent_is_set(id: &str) -> bool { load_settings()["palette_accent"][id].as_str().is_some() }

/// A guess at the dark / light of the system for Match system (egui reports it; Linux with no portal says dark).
pub fn is_light_system(ctx: &eframe::egui::Context) -> bool {
    ctx.input(|i| i.raw.system_theme) == Some(eframe::egui::Theme::Light)
}

pub fn _unused(_: Value) {}
