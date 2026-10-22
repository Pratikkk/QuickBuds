//! The four styles of the app: Glass (the default), Material, QuickBuds (classic) and Pixel art. The widgets read the
//! current style and palette from here, so every page follows a change at once.

use crate::palette::Pal;
use eframe::egui::Color32;
use std::sync::RwLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style { Glass, Material, Classic, Pixel }

impl Style {
    pub const ALL: [Style; 4] = [Style::Glass, Style::Material, Style::Classic, Style::Pixel];
    pub fn from_key(k: &str) -> Style {
        match k { "material" => Style::Material, "classic" => Style::Classic, "pixel" => Style::Pixel, _ => Style::Glass }
    }
    pub fn key(self) -> &'static str {
        match self { Style::Glass => "glass", Style::Material => "material", Style::Classic => "classic", Style::Pixel => "pixel" }
    }
    /// The Android string for the style's name (`widget_style_*`).
    pub fn name_key(self) -> &'static str {
        match self { Style::Glass => "widget_style_glass", Style::Material => "widget_style_material", Style::Classic => "widget_style_classic", Style::Pixel => "widget_style_nothing" }
    }
}

static CURRENT: RwLock<(Style, Pal)> = RwLock::new((Style::Glass, Pal::DEFAULT));

pub fn set(style: Style, pal: Pal) { *CURRENT.write().unwrap() = (style, pal); }
pub fn style() -> Style { CURRENT.read().unwrap().0 }
pub fn pal() -> Pal { CURRENT.read().unwrap().1 }
pub fn glass() -> bool { style() == Style::Glass }

/// The text and icon colour at opacity `a` (1 = full, lower = secondary). Glass is white on its dark window.
pub fn ink(a: f32) -> Color32 {
    let (s, p) = *CURRENT.read().unwrap();
    let base = if s == Style::Glass { Color32::WHITE } else if a < 0.85 { crate::palette::blend(p.text, p.text2, ((0.85 - a) * 4.0).min(1.0)) } else { p.text };
    let alpha = if s == Style::Glass || a >= 0.85 { a } else { (0.6 + a * 0.4).min(1.0) };
    Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), (alpha.clamp(0.0, 1.0) * 255.0) as u8)
}

/// The accent fill of a selection, and the colour of a label on it.
pub fn accent() -> Color32 { CURRENT.read().unwrap().1.accent }

pub fn on_accent() -> Color32 {
    let (s, p) = *CURRENT.read().unwrap();
    if s == Style::Glass { Color32::from_rgb(27, 26, 69) } else { p.on_accent() }
}
