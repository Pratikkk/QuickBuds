//! The UI font. The glass design is set in a clean humanist sans (SF Pro / Inter in the artifact). egui's own default
//! is a light Ubuntu cut; this puts the first of these system fonts in front when the PC has one. The other scripts
//! come from egui's system font fallback.

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// In order of preference.
const CANDIDATES: &[&str] = &[
    // Windows 11: Segoe UI Variable, then Segoe UI.
    r"C:\Windows\Fonts\SegUIVar.ttf",
    r"C:\Windows\Fonts\segoeui.ttf",
    // Linux
    "/usr/share/fonts/inter/Inter-Regular.ttf",
    "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
    "/usr/share/fonts/TTF/Inter-Regular.ttf",
    "/usr/share/fonts/inter/InterVariable.ttf",
    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/google-noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/cantarell/Cantarell-VF.otf",
];

pub fn install(ctx: &egui::Context) {
    let Some(bytes) = CANDIDATES.iter().find_map(|p| std::fs::read(p).ok()) else { return };
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("ui".into(), Arc::new(FontData::from_owned(bytes)));
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "ui".into());
    ctx.set_fonts(fonts);
}
