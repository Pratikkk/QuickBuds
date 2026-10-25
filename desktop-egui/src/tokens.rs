//! The colour tokens of the phone app, ported from `ui/Palette.kt` and `ui/ThemeRes.kt`.
//!
//! The six theme tokens live in `values/themes.xml` (`Theme.App.OLED`, `.Dark`, `.Light`).
//! Every other colour is derived from them. This module does the derivation with the same
//! rules as the phone, and with the same alpha: a Glass surface keeps its see-through fill.
//!
//! The rules:
//! - light / dark follows WCAG relative luminance of the background.
//! - `on_accent` picks the label that reads on an accent fill. It prefers 3:1.
//! - `container` lifts a pure black card, then tints it with the accent.
//! - Glass replaces the six tokens with translucent whites and deep ink.

use crate::glass::style::Style;
use eframe::egui::Color32;

/// One colour with straight (not premultiplied) alpha, in the phone's ARGB order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const TRANSPARENT: Rgba = Rgba::argb(0, 0, 0, 0);
    pub const WHITE: Rgba = Rgba::rgb(255, 255, 255);

    /// `#RRGGBB`, or `#AARRGGBB` when the colour is see-through.
    pub fn hex(self) -> String {
        if self.a == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.a, self.r, self.g, self.b)
        }
    }

    pub fn color32(self) -> Color32 {
        Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }
}

/// WCAG 2 relative luminance, 0..1.
pub fn luminance(c: Rgba) -> f64 {
    let ch = |v: u8| {
        let s = v as f64 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(c.r) + 0.7152 * ch(c.g) + 0.0722 * ch(c.b)
}

/// WCAG 2 contrast ratio, 1..21.
pub fn contrast(a: Rgba, b: Rgba) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Interpolates every channel, alpha included, like `Palette.blend`.
pub fn blend(from: Rgba, to: Rgba, t: f32) -> Rgba {
    let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Rgba {
        r: m(from.r, to.r),
        g: m(from.g, to.g),
        b: m(from.b, to.b),
        a: m(from.a, to.a),
    }
}

/// `Palette.withAlpha`: sets alpha to `a * 255`, truncated.
pub fn with_alpha(c: Rgba, a: f32) -> Rgba {
    Rgba {
        a: (a * 255.0) as u8,
        ..c
    }
}

/// The six theme tokens of one preset.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Palette {
    pub bg: Rgba,
    pub card: Rgba,
    pub accent: Rgba,
    pub text: Rgba,
    pub text2: Rgba,
    pub outline: Rgba,
}

impl Palette {
    pub fn is_light(self) -> bool {
        luminance(self.bg) > 0.5
    }

    /// Material: the card's tonal surface. A pure black card lifts toward the text first.
    pub fn container(self) -> Rgba {
        let lift = if !self.is_light() && luminance(self.card) < 0.005 {
            0.09
        } else {
            0.0
        };
        blend(
            blend(self.card, self.text, lift),
            self.accent,
            if self.is_light() { 0.05 } else { 0.08 },
        )
    }

    /// Material: a selected element's fill. Its label is the text token.
    pub fn tonal(self) -> Rgba {
        blend(
            self.container(),
            self.accent,
            if self.is_light() { 0.18 } else { 0.28 },
        )
    }

    /// The label on an accent fill: the lighter token when it reaches 3:1, else the better one.
    pub fn on_accent(self) -> Rgba {
        let (light, dark) = if luminance(self.text) > luminance(self.bg) {
            (self.text, self.bg)
        } else {
            (self.bg, self.text)
        };
        if contrast(light, self.accent) >= 3.0
            || contrast(light, self.accent) >= contrast(dark, self.accent)
        {
            light
        } else {
            dark
        }
    }

    /// Switch track: the outline, moved toward the text.
    pub fn track(self) -> Rgba {
        blend(
            self.outline,
            self.text,
            if self.is_light() { 0.14 } else { 0.06 },
        )
    }

    /// Switch thumb when off: the text token on dark themes, the card on light ones.
    pub fn thumb_off(self) -> Rgba {
        if self.is_light() {
            self.card
        } else {
            with_alpha(self.text, 0.9)
        }
    }

    /// Empty rings and placeholder glyphs.
    pub fn disabled(self) -> Rgba {
        with_alpha(self.text2, 0.5)
    }

    /// Background of an expanded row.
    pub fn expanded(self) -> Rgba {
        blend(self.card, self.text, 0.05)
    }
}

/// One built-in theme, as `values/themes.xml` declares it.
pub struct Theme {
    pub id: &'static str,
    pub name_key: &'static str,
    pub palette: Palette,
}

pub const OLED: &str = "oled";
pub const DARK: &str = "dark";
pub const WHITE: &str = "white";

pub const THEMES: [Theme; 3] = [
    Theme {
        id: OLED,
        name_key: "theme_oled_black",
        palette: Palette {
            bg: Rgba::rgb(0x00, 0x00, 0x00),
            card: Rgba::rgb(0x00, 0x00, 0x00),
            accent: Rgba::rgb(0xD7, 0x19, 0x20),
            text: Rgba::rgb(0xFF, 0xFF, 0xFF),
            text2: Rgba::rgb(0x9B, 0x9B, 0x9B),
            outline: Rgba::rgb(0x2E, 0x2E, 0x30),
        },
    },
    Theme {
        id: DARK,
        name_key: "theme_classic_dark",
        palette: Palette {
            bg: Rgba::rgb(0x12, 0x12, 0x14),
            card: Rgba::rgb(0x23, 0x23, 0x27),
            accent: Rgba::rgb(0xD7, 0x19, 0x20),
            text: Rgba::rgb(0xFF, 0xFF, 0xFF),
            text2: Rgba::rgb(0xA0, 0xA0, 0xA5),
            outline: Rgba::rgb(0x34, 0x34, 0x3A),
        },
    },
    Theme {
        id: WHITE,
        name_key: "theme_white",
        palette: Palette {
            bg: Rgba::rgb(0xEC, 0xEC, 0xF0),
            card: Rgba::rgb(0xFF, 0xFF, 0xFF),
            accent: Rgba::rgb(0xD7, 0x19, 0x20),
            text: Rgba::rgb(0x17, 0x17, 0x1A),
            text2: Rgba::rgb(0x6A, 0x6A, 0x72),
            outline: Rgba::rgb(0xDC, 0xDC, 0xE2),
        },
    },
];

pub fn theme(id: &str) -> &'static Theme {
    THEMES.iter().find(|t| t.id == id).unwrap_or(&THEMES[0])
}

/// Glass: the label on an accent fill. The window is not a token, so white ink or the deep ink.
pub fn glass_on_accent(accent: Rgba) -> Rgba {
    if contrast(Rgba::WHITE, accent) >= 3.0 {
        Rgba::WHITE
    } else {
        Rgba::rgb(0x14, 0x15, 0x2B)
    }
}

/// The Glass backgrounds, in the phone picker's order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GlassBg {
    Midnight,
    Aurora,
    Sunset,
    Ocean,
    Accent,
    None,
}

impl GlassBg {
    pub const ALL: [GlassBg; 6] = [
        GlassBg::Midnight,
        GlassBg::Aurora,
        GlassBg::Sunset,
        GlassBg::Ocean,
        GlassBg::Accent,
        GlassBg::None,
    ];

    pub fn name_key(self) -> &'static str {
        match self {
            GlassBg::Midnight => "glass_bg_midnight",
            GlassBg::Aurora => "glass_bg_aurora",
            GlassBg::Sunset => "glass_bg_sunset",
            GlassBg::Ocean => "glass_bg_ocean",
            GlassBg::Accent => "glass_bg_accent",
            GlassBg::None => "glass_bg_none",
        }
    }

    fn fixed(self) -> Option<[Rgba; 4]> {
        match self {
            GlassBg::Midnight => Some([
                Rgba::rgb(0x4A, 0x6D, 0xFF),
                Rgba::rgb(0xC2, 0x7B, 0xFF),
                Rgba::rgb(0xFF, 0x9D, 0x6B),
                Rgba::rgb(0x27, 0xD0, 0xC4),
            ]),
            GlassBg::Aurora => Some([
                Rgba::rgb(0x2B, 0xD9, 0x9B),
                Rgba::rgb(0x3B, 0x82, 0xF6),
                Rgba::rgb(0x8B, 0x5C, 0xF6),
                Rgba::rgb(0x22, 0xD3, 0xEE),
            ]),
            GlassBg::Sunset => Some([
                Rgba::rgb(0xFF, 0x6B, 0x6B),
                Rgba::rgb(0xFF, 0xA9, 0x4D),
                Rgba::rgb(0xC2, 0x41, 0x0C),
                Rgba::rgb(0xEC, 0x48, 0x99),
            ]),
            GlassBg::Ocean => Some([
                Rgba::rgb(0x0E, 0xA5, 0xE9),
                Rgba::rgb(0x1D, 0x4E, 0xD8),
                Rgba::rgb(0x14, 0xB8, 0xA6),
                Rgba::rgb(0x38, 0xBD, 0xF8),
            ]),
            GlassBg::Accent | GlassBg::None => None,
        }
    }

    /// `None` is the flat window; the Accent set is built from the accent token.
    pub fn blobs(self, accent: Rgba) -> Option<[Rgba; 4]> {
        if self == GlassBg::None {
            return None;
        }
        Some(self.fixed().unwrap_or([
            accent,
            blend(accent, Rgba::rgb(0x4A, 0x6D, 0xFF), 0.5),
            blend(accent, Rgba::rgb(0xFF, 0x9D, 0x6B), 0.5),
            blend(accent, Rgba::WHITE, 0.35),
        ]))
    }
}

/// The window behind the glass: a base, four colour blobs and their alpha.
pub struct Window {
    pub base: Rgba,
    pub blobs: Option<[Rgba; 4]>,
    pub alphas: [f32; 4],
    /// Blob centres as a fraction of the window: x 0..1 from the left, y 0..1 from the top.
    pub centers: [(f32, f32); 4],
    pub gradient: bool,
}

pub const BLOB_CENTERS: [(f32, f32); 4] = [(0.12, 0.18), (0.88, 0.12), (0.78, 0.92), (0.08, 0.95)];

/// `ThemeRes.glassBackground`, as data.
pub fn window(preset: Palette, bg: GlassBg) -> Window {
    let light = preset.is_light();
    let oled = luminance(preset.bg) < 0.005;
    if bg == GlassBg::None {
        let base = if light {
            Rgba::rgb(0xF4, 0xF5, 0xFA)
        } else if oled {
            Rgba::rgb(0x00, 0x00, 0x00)
        } else {
            Rgba::rgb(0x1B, 0x1D, 0x26)
        };
        return Window {
            base,
            blobs: None,
            alphas: [0.0; 4],
            centers: BLOB_CENTERS,
            gradient: false,
        };
    }
    let base = if light {
        Rgba::rgb(0xF1, 0xF3, 0xFA)
    } else if oled {
        Rgba::rgb(0x05, 0x05, 0x09)
    } else {
        Rgba::rgb(0x1B, 0x1D, 0x2B)
    };
    let alphas = if light {
        [0.34, 0.28, 0.24, 0.26]
    } else if oled {
        [0.20, 0.14, 0.11, 0.12]
    } else {
        [0.26, 0.19, 0.15, 0.17]
    };
    Window {
        base,
        blobs: bg.blobs(preset.accent),
        alphas,
        centers: BLOB_CENTERS,
        gradient: true,
    }
}

/// Every token of one style and theme, after the Glass transform.
pub struct Tokens {
    pub style: Style,
    pub theme: &'static Theme,
    pub glass: bool,
    pub glass_light: bool,
    pub bg: Rgba,
    pub card: Rgba,
    pub accent: Rgba,
    pub text: Rgba,
    pub text2: Rgba,
    pub outline: Rgba,
    pub container: Rgba,
    pub tonal: Rgba,
    pub on_accent: Rgba,
    /// The label on a fill: Pixel art outlines its fills, so it uses the accent itself.
    pub label_on_accent: Rgba,
    /// Material: a selected list item's 24% accent tint.
    pub selected: Rgba,
    pub track: Rgba,
    pub thumb_off: Rgba,
    pub disabled: Rgba,
    pub expanded: Rgba,
    /// Glass: the ink for surfaces, tracks and rims.
    pub ink: Rgba,
    /// Glass: a selected surface, `GLASS_SELECTED`.
    pub glass_selected: Rgba,
    /// Glass: the window fill multiplies its alpha by 0.7 over the gradient, 0.45 over black.
    pub glass_alpha: f32,
    pub window: Window,
}

impl Tokens {
    pub fn new(style: Style, theme: &'static Theme, bg: GlassBg) -> Self {
        let base = theme.palette;
        let glass_light = base.is_light();
        // `ThemeRes.glassify`: deep gradient, translucent whites, deep ink on a light preset.
        let palette = if style == Style::Glass {
            Palette {
                bg: Rgba::TRANSPARENT,
                card: if glass_light {
                    Rgba::argb(0x66, 0xFF, 0xFF, 0xFF)
                } else {
                    Rgba::argb(0x1A, 0xFF, 0xFF, 0xFF)
                },
                accent: base.accent,
                text: if glass_light {
                    Rgba::rgb(0x14, 0x15, 0x2B)
                } else {
                    Rgba::WHITE
                },
                text2: if glass_light {
                    Rgba::argb(0xB3, 0x14, 0x15, 0x2B)
                } else {
                    Rgba::argb(0xB8, 0xFF, 0xFF, 0xFF)
                },
                outline: if glass_light {
                    Rgba::argb(0x29, 0x14, 0x15, 0x2B)
                } else {
                    Rgba::argb(0x59, 0xFF, 0xFF, 0xFF)
                },
            }
        } else {
            base
        };
        let glass = style == Style::Glass;
        let container = if glass {
            if glass_light {
                Rgba::argb(0x80, 0xFF, 0xFF, 0xFF)
            } else {
                Rgba::argb(0x1A, 0xFF, 0xFF, 0xFF)
            }
        } else {
            palette.container()
        };
        let tonal = if glass {
            with_alpha(palette.accent, 0.4)
        } else {
            palette.tonal()
        };
        let on_accent = if glass {
            glass_on_accent(palette.accent)
        } else {
            palette.on_accent()
        };
        let label_on_accent = if style == Style::Pixel {
            palette.accent
        } else {
            on_accent
        };
        Self {
            style,
            theme,
            glass,
            glass_light,
            bg: palette.bg,
            card: palette.card,
            accent: palette.accent,
            text: palette.text,
            text2: palette.text2,
            outline: palette.outline,
            container,
            tonal,
            on_accent,
            label_on_accent,
            selected: with_alpha(palette.accent, 0.24),
            // Light / dark comes from the preset: a Glass fill is transparent and cannot tell it.
            track: blend(
                palette.outline,
                palette.text,
                if glass_light { 0.14 } else { 0.06 },
            ),
            thumb_off: if glass_light {
                palette.card
            } else {
                with_alpha(palette.text, 0.9)
            },
            disabled: palette.disabled(),
            expanded: palette.expanded(),
            ink: if glass_light {
                Rgba::rgb(0x14, 0x15, 0x2B)
            } else {
                Rgba::WHITE
            },
            glass_selected: Rgba::argb(0x38, 0xFF, 0xFF, 0xFF),
            glass_alpha: if bg == GlassBg::None { 0.45 } else { 0.7 },
            window: window(base, bg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THEMES_XML: &str = include_str!("../../app/src/main/res/values/themes.xml");

    /// The `<item>` value of one token inside one `<style>`.
    fn xml_token(style: &str, token: &str) -> String {
        let start = THEMES_XML
            .find(&format!("<style name=\"{style}\""))
            .expect(style);
        let end = start + THEMES_XML[start..].find("</style>").unwrap();
        let block = &THEMES_XML[start..end];
        let key = format!("<item name=\"{token}\">");
        let at = block.find(&key).unwrap() + key.len();
        block[at..at + block[at..].find('<').unwrap()]
            .trim()
            .to_string()
    }

    fn hx(c: Rgba) -> String {
        c.hex()
    }

    fn all() -> Vec<(Style, &'static Theme)> {
        let mut v = Vec::new();
        for s in Style::ALL {
            for t in &THEMES {
                v.push((s, t));
            }
        }
        v
    }

    #[test]
    fn built_in_tokens_match_themes_xml() {
        let xml = [
            ("Theme.App.OLED", OLED),
            ("Theme.App.Dark", DARK),
            ("Theme.App.Light", WHITE),
        ];
        for (style, id) in xml {
            let p = theme(id).palette;
            assert_eq!(hx(p.bg), xml_token(style, "appColorBg"), "{id} background");
            assert_eq!(hx(p.card), xml_token(style, "appColorCard"), "{id} card");
            assert_eq!(
                hx(p.accent),
                xml_token(style, "appColorAccent"),
                "{id} accent"
            );
            assert_eq!(
                hx(p.text),
                xml_token(style, "appColorTextPrimary"),
                "{id} text"
            );
            assert_eq!(
                hx(p.text2),
                xml_token(style, "appColorTextSecondary"),
                "{id} secondary text"
            );
            assert_eq!(
                hx(p.outline),
                xml_token(style, "appColorOutline"),
                "{id} outline"
            );
        }
    }

    #[test]
    fn light_and_dark_follow_the_background() {
        assert!(!theme(OLED).palette.is_light());
        assert!(!theme(DARK).palette.is_light());
        assert!(theme(WHITE).palette.is_light());
    }

    #[test]
    fn oled_derived_tokens() {
        let p = theme(OLED).palette;
        assert_eq!(hx(p.container()), "#251616");
        assert_eq!(hx(p.tonal()), "#561618");
        assert_eq!(hx(p.track()), "#3A3A3C");
        assert_eq!(hx(p.on_accent()), "#FFFFFF");
        assert_eq!(hx(p.thumb_off()), "#E5FFFFFF");
        assert_eq!(hx(p.disabled()), "#7F9B9B9B");
        assert_eq!(hx(p.expanded()), "#0C0C0C");
    }

    #[test]
    fn classic_dark_derived_tokens() {
        let p = theme(DARK).palette;
        assert_eq!(hx(p.container()), "#312226");
        assert_eq!(hx(p.tonal()), "#5F1F24");
        assert_eq!(hx(p.track()), "#404045");
        assert_eq!(hx(p.on_accent()), "#FFFFFF");
        assert_eq!(hx(p.thumb_off()), "#E5FFFFFF");
        assert_eq!(hx(p.disabled()), "#7FA0A0A5");
        assert_eq!(hx(p.expanded()), "#2E2E31");
    }

    #[test]
    fn white_derived_tokens() {
        let p = theme(WHITE).palette;
        assert_eq!(hx(p.container()), "#FDF3F3");
        assert_eq!(hx(p.tonal()), "#F6CBCD");
        assert_eq!(hx(p.track()), "#C0C0C6");
        // On a light preset the lighter token is the page, not the text, so the label uses the page
        // colour. It reaches 4.40:1 on the accent, above the 3:1 minimum.
        assert_eq!(hx(p.on_accent()), "#ECECF0");
        assert_eq!(hx(p.thumb_off()), "#FFFFFF");
        assert_eq!(hx(p.disabled()), "#7F6A6A72");
        assert_eq!(hx(p.expanded()), "#F3F3F3");
    }

    #[test]
    fn contrast_uses_wcag_luminance() {
        let white = Rgba::WHITE;
        let black = Rgba::rgb(0, 0, 0);
        let red = theme(OLED).palette.accent;
        assert!((contrast(white, black) - 21.0).abs() < 0.01);
        assert!((contrast(white, white) - 1.0).abs() < 0.01);
        assert!((contrast(white, red) - 5.19).abs() < 0.01);
        assert!((contrast(black, red) - 4.05).abs() < 0.01);
        // White text is readable on every built-in background.
        for t in &THEMES {
            assert!(contrast(t.palette.text, t.palette.bg) >= 4.5, "{}", t.id);
        }
    }

    #[test]
    fn accent_contrast_decides_the_label() {
        let red = theme(OLED).palette.accent;
        let gold = Rgba::rgb(0xFF, 0xD7, 0x00);
        // Glass: white reads on red, the deep ink reads on gold.
        assert_eq!(
            hx(Tokens::new(Style::Glass, theme(OLED), GlassBg::Midnight).on_accent),
            "#FFFFFF"
        );
        let light_glass = Tokens::new(Style::Glass, theme(WHITE), GlassBg::Midnight);
        assert_eq!(hx(light_glass.on_accent), "#FFFFFF");
        assert_eq!(hx(glass_on_accent(red)), "#FFFFFF");
        assert_eq!(hx(glass_on_accent(gold)), "#14152B");
    }

    #[test]
    fn glass_dark_tokens() {
        let t = Tokens::new(Style::Glass, theme(DARK), GlassBg::Midnight);
        assert!(t.glass);
        assert!(!t.glass_light);
        assert_eq!(hx(t.bg), "#00000000");
        assert_eq!(hx(t.card), "#1AFFFFFF");
        assert_eq!(hx(t.text), "#FFFFFF");
        assert_eq!(hx(t.text2), "#B8FFFFFF");
        assert_eq!(hx(t.outline), "#59FFFFFF");
        assert_eq!(hx(t.container), "#1AFFFFFF");
        assert_eq!(hx(t.glass_selected), "#38FFFFFF");
        assert_eq!(hx(t.tonal), "#66D71920");
        assert_eq!(hx(t.ink), "#FFFFFF");
        assert_eq!(t.glass_alpha, 0.7);
    }

    #[test]
    fn glass_light_tokens() {
        let t = Tokens::new(Style::Glass, theme(WHITE), GlassBg::Midnight);
        assert!(t.glass_light);
        assert_eq!(hx(t.card), "#66FFFFFF");
        assert_eq!(hx(t.text), "#14152B");
        assert_eq!(hx(t.text2), "#B314152B");
        assert_eq!(hx(t.outline), "#2914152B");
        assert_eq!(hx(t.container), "#80FFFFFF");
        assert_eq!(hx(t.ink), "#14152B");
        assert_eq!(hx(t.thumb_off), "#66FFFFFF");
    }

    #[test]
    fn glass_flat_window_changes_the_alpha_rule() {
        let t = Tokens::new(Style::Glass, theme(OLED), GlassBg::None);
        assert_eq!(t.glass_alpha, 0.45);
        assert_eq!(hx(t.window.base), "#000000");
        assert!(t.window.blobs.is_none());
        assert!(!t.window.gradient);
    }

    #[test]
    fn glass_backgrounds_use_the_phone_colours() {
        let p = theme(DARK).palette;
        let midnight = window(p, GlassBg::Midnight);
        assert!(midnight.gradient);
        assert_eq!(hx(midnight.base), "#1B1D2B");
        assert_eq!(midnight.alphas, [0.26, 0.19, 0.15, 0.17]);
        let blobs = midnight.blobs.unwrap();
        assert_eq!(hx(blobs[0]), "#4A6DFF");
        assert_eq!(hx(blobs[3]), "#27D0C4");
        // The same background over OLED Black and over White keeps the design's base and alpha.
        assert_eq!(
            hx(window(theme(OLED).palette, GlassBg::Midnight).base),
            "#050509"
        );
        assert_eq!(
            window(theme(OLED).palette, GlassBg::Midnight).alphas,
            [0.20, 0.14, 0.11, 0.12]
        );
        assert_eq!(
            hx(window(theme(WHITE).palette, GlassBg::Midnight).base),
            "#F1F3FA"
        );
        assert_eq!(
            window(theme(WHITE).palette, GlassBg::Midnight).alphas,
            [0.34, 0.28, 0.24, 0.26]
        );
    }

    #[test]
    fn accent_background_tints_from_the_accent() {
        let red = theme(OLED).palette.accent;
        let blobs = Window {
            base: Rgba::TRANSPARENT,
            blobs: GlassBg::Accent.blobs(red),
            alphas: [0.0; 4],
            centers: BLOB_CENTERS,
            gradient: true,
        }
        .blobs
        .unwrap();
        assert_eq!(hx(blobs[0]), "#D71920");
        assert_eq!(hx(blobs[1]), "#90438F");
        assert_eq!(hx(blobs[2]), "#EB5B45");
        assert_eq!(hx(blobs[3]), "#E5696E");
    }

    #[test]
    fn alpha_is_kept_and_truncated_like_the_phone() {
        assert_eq!(with_alpha(Rgba::WHITE, 0.9).a, 229);
        assert_eq!(with_alpha(Rgba::WHITE, 0.4).a, 102);
        assert_eq!(with_alpha(Rgba::WHITE, 0.24).a, 61);
        assert_eq!(
            with_alpha(Rgba::rgb(0x4A, 0x6D, 0xFF), 0.26).hex(),
            "#424A6DFF"
        );
        // blend interpolates alpha, so a glass surface keeps its fill.
        let glass = blend(Rgba::argb(0x1A, 0xFF, 0xFF, 0xFF), Rgba::WHITE, 0.5);
        assert_eq!(glass.a, 140);
    }

    #[test]
    fn every_style_and_theme_gives_every_token() {
        for (style, th) in all() {
            for bg in GlassBg::ALL {
                let t = Tokens::new(style, th, bg);
                let tokens = [
                    t.bg,
                    t.card,
                    t.accent,
                    t.text,
                    t.text2,
                    t.outline,
                    t.container,
                    t.tonal,
                    t.on_accent,
                    t.label_on_accent,
                    t.selected,
                    t.track,
                    t.thumb_off,
                    t.disabled,
                    t.expanded,
                    t.ink,
                    t.window.base,
                ];
                for c in tokens {
                    assert!(c.hex().len() >= 7, "{:?}/{}", style, th.id);
                }
                // The six tokens and their tonal surfaces stay opaque without Glass.
                if !t.glass {
                    for c in [
                        t.bg,
                        t.card,
                        t.accent,
                        t.text,
                        t.text2,
                        t.outline,
                        t.container,
                        t.tonal,
                        t.on_accent,
                        t.window.base,
                    ] {
                        assert_eq!(c.a, 255, "{:?}/{}", style, th.id);
                    }
                }
            }
        }
    }

    #[test]
    fn pixel_art_labels_its_fills_with_the_accent() {
        let t = Tokens::new(Style::Pixel, theme(OLED), GlassBg::Midnight);
        assert_eq!(hx(t.label_on_accent), "#D71920");
        let m = Tokens::new(Style::Material, theme(OLED), GlassBg::Midnight);
        assert_eq!(hx(m.label_on_accent), "#FFFFFF");
        assert_eq!(hx(m.selected), "#3DD71920");
    }

    #[test]
    fn strings_cover_every_locale_and_fall_back_to_english() {
        assert!(
            crate::strings::LOCALES.len() >= 27,
            "English plus 26 translations"
        );
        let en = crate::translations("");
        let de = crate::translations("de");
        let unknown = crate::translations("zz");
        assert_eq!(en.len(), crate::strings::KEYS.len());
        assert_eq!(de.len(), en.len());
        assert_eq!(unknown, en);
        for key in [
            "token_background",
            "token_card",
            "token_accent",
            "token_text",
            "token_text_secondary",
            "token_outline",
        ] {
            let at = crate::strings::KEYS.iter().position(|k| *k == key).unwrap();
            assert!(!en[at].is_empty(), "{key}");
        }
    }
}
