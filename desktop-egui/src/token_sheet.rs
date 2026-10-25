//! The token sheet window: every token of one style and theme, in labelled rows.
//!
//! Run it with `cargo run -- --tokens`. The window does not link to the earbuds and does not
//! read saved settings, so it always shows the built-in values from `values/themes.xml`.

use crate::glass::style::Style;
use crate::tokens::{self, contrast, with_alpha, GlassBg, Rgba, Tokens};
use crate::{fonts, t};
use eframe::egui::{self, RichText};

const PAD: i8 = 16;
const GAP: f32 = 8.0;

pub struct TokenSheet {
    style: Style,
    theme: usize,
    glass_bg: GlassBg,
    tr: Vec<String>,
}

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 760.0])
            .with_min_inner_size([560.0, 480.0])
            .with_title("QuickBuds tokens"),
        ..Default::default()
    };
    eframe::run_native(
        "QuickBuds tokens",
        options,
        Box::new(|cc| Ok(Box::new(TokenSheet::new(cc)))),
    )
}

impl TokenSheet {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        fonts::apply(&cc.egui_ctx, Style::Glass);
        Self {
            style: Style::Glass,
            theme: 0,
            glass_bg: GlassBg::Midnight,
            tr: crate::translations(&crate::chosen_locale()),
        }
    }

    fn tokens(&self) -> Tokens {
        Tokens::new(self.style, &tokens::THEMES[self.theme], self.glass_bg)
    }
}

impl eframe::App for TokenSheet {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The sheet sits on the same window as the style it describes.
        let tok = self.tokens();
        let dark = if tok.glass {
            !tok.glass_light
        } else {
            !tok.theme.palette.is_light()
        };
        let mut visuals = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        let page = if tok.glass { tok.window.base } else { tok.bg };
        visuals.override_text_color = Some(if tok.glass { tok.ink } else { tok.text }.color32());
        visuals.panel_fill = page.color32();
        visuals.window_fill = page.color32();
        ui.ctx().set_visuals(visuals);

        let full = ui.max_rect();
        // A Glass sheet sits on its window. Every other style sits on its page colour.
        let p = ui.painter();
        p.rect_filled(full, 0.0, page.color32());
        if tok.glass {
            if let Some(blobs) = tok.window.blobs {
                let radius = full.height() * 0.5;
                // Bands approximate the phone's radial fade: strong at the centre, clear at the rim.
                const BANDS: usize = 40;
                let width = radius / BANDS as f32;
                for (i, c) in blobs.iter().enumerate() {
                    let (cx, cy) = tok.window.centers[i];
                    let center = full.lerp_inside(egui::vec2(cx, cy));
                    for band in 0..BANDS {
                        let f = 1.0 - band as f32 / BANDS as f32;
                        let r = radius * f - width / 2.0;
                        let a = tok.window.alphas[i] * f * f;
                        p.circle_stroke(
                            center,
                            r,
                            egui::Stroke::new(width, with_alpha(*c, a).color32()),
                        );
                    }
                }
            }
        }

        egui::Frame::new()
            .inner_margin(egui::Margin::same(PAD))
            .show(ui, |ui| {
                let ink = if tok.glass { tok.ink } else { tok.text };
                ui.heading(RichText::new("QuickBuds tokens").color(ink.color32()));
                ui.add_space(GAP * 0.5);
                ui.label("Each row shows one token. Compare a row with the same row on the phone.");
                ui.add_space(GAP);

                // Style: four mutually exclusive choices, in the phone picker's order.
                ui.horizontal_wrapped(|ui| {
                    for s in [Style::Material, Style::Classic, Style::Pixel, Style::Glass] {
                        let name = t(&self.tr, s.name_key()).to_string();
                        if ui.selectable_value(&mut self.style, s, name).changed() {
                            // Pixel art sets Doto for the whole window; the others use the system sans.
                            fonts::apply(ui.ctx(), self.style);
                        }
                    }
                });
                ui.add_space(GAP * 0.5);
                ui.horizontal_wrapped(|ui| {
                    for i in 0..tokens::THEMES.len() {
                        let name = t(&self.tr, tokens::THEMES[i].name_key).to_string();
                        ui.selectable_value(&mut self.theme, i, name);
                    }
                });
                if self.style == Style::Glass {
                    ui.add_space(GAP * 0.5);
                    ui.horizontal_wrapped(|ui| {
                        for bg in GlassBg::ALL {
                            let name = t(&self.tr, bg.name_key()).to_string();
                            ui.selectable_value(&mut self.glass_bg, bg, name);
                        }
                    });
                }
                ui.add_space(GAP);

                let tok = self.tokens();
                let ink = if tok.glass { tok.ink } else { tok.text };
                let light = if tok.glass {
                    tok.glass_light
                } else {
                    tok.theme.palette.is_light()
                };
                ui.label(
                    RichText::new(format!(
                        "{:?} / {} / {}",
                        tok.style,
                        t(&self.tr, tok.theme.name_key),
                        if light { "light" } else { "dark" }
                    ))
                    .color(ink.color32())
                    .strong(),
                );
                ui.add_space(GAP);
                preview(ui, &tok, ink);
                ui.add_space(GAP);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("token_rows")
                            .num_columns(4)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                for item in items(&tok, &self.tr) {
                                    swatch(ui, item.value, page);
                                    ui.label(RichText::new(item.label).color(ink.color32()));
                                    ui.monospace(
                                        RichText::new(item.value.hex()).color(ink.color32()),
                                    );
                                    match item.ratio {
                                        Some((r, min)) => {
                                            let low = r < min;
                                            ui.label(
                                                RichText::new(format!(
                                                    "{r:.2}:1{}",
                                                    if low { " low" } else { "" }
                                                ))
                                                .color(ink.color32()),
                                            );
                                        }
                                        None => {
                                            ui.label("");
                                        }
                                    }
                                    ui.end_row();
                                }
                            });

                        if tok.glass {
                            ui.add_space(GAP * 2.0);
                            ui.label(RichText::new("Glass window").color(ink.color32()).strong());
                            ui.add_space(GAP * 0.5);
                            egui::Grid::new("window_rows")
                                .num_columns(2)
                                .spacing([12.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label(RichText::new("window base").color(ink.color32()));
                                    ui.monospace(
                                        RichText::new(tok.window.base.hex()).color(ink.color32()),
                                    );
                                    ui.end_row();
                                    if let Some(blobs) = tok.window.blobs {
                                        for (i, c) in blobs.iter().enumerate() {
                                            ui.label(
                                                RichText::new(format!(
                                                    "blob {} at {:.2} / {:.2}",
                                                    i + 1,
                                                    tok.window.centers[i].0,
                                                    tok.window.centers[i].1
                                                ))
                                                .color(ink.color32()),
                                            );
                                            ui.monospace(
                                                RichText::new(
                                                    with_alpha(*c, tok.window.alphas[i]).hex(),
                                                )
                                                .color(ink.color32()),
                                            );
                                            ui.end_row();
                                        }
                                    }
                                    ui.label(
                                        RichText::new("window fill alpha").color(ink.color32()),
                                    );
                                    ui.monospace(
                                        RichText::new(format!("{:.2}", tok.glass_alpha))
                                            .color(ink.color32()),
                                    );
                                    ui.end_row();
                                    ui.label(RichText::new("gradient").color(ink.color32()));
                                    ui.monospace(
                                        RichText::new(if tok.window.gradient {
                                            "on"
                                        } else {
                                            "off"
                                        })
                                        .color(ink.color32()),
                                    );
                                    ui.end_row();
                                });
                        }
                    });
            });
    }
}

struct Item {
    label: String,
    value: Rgba,
    /// Contrast against its surface, and the WCAG minimum for it.
    ratio: Option<(f64, f64)>,
}

fn items(tok: &Tokens, tr: &[String]) -> Vec<Item> {
    // Glass tokens are see-through: check the contrast of the painted result on the window.
    let board = if tok.glass { tok.window.base } else { tok.bg };
    let painted = |c: Rgba| over(c, board);
    let mut v = vec![
        Item {
            label: t(tr, "token_background").into(),
            value: tok.bg,
            ratio: None,
        },
        Item {
            label: t(tr, "token_card").into(),
            value: tok.card,
            ratio: None,
        },
        Item {
            label: t(tr, "token_accent").into(),
            value: tok.accent,
            ratio: None,
        },
        Item {
            label: t(tr, "token_text").into(),
            value: tok.text,
            ratio: Some((contrast(painted(tok.text), board), 4.5)),
        },
        Item {
            label: t(tr, "token_text_secondary").into(),
            value: tok.text2,
            ratio: Some((contrast(painted(tok.text2), board), 4.5)),
        },
        Item {
            label: t(tr, "token_outline").into(),
            value: tok.outline,
            ratio: None,
        },
    ];
    if tok.style == Style::Material {
        v.push(Item {
            label: "container".into(),
            value: tok.container,
            ratio: None,
        });
        v.push(Item {
            label: "tonal".into(),
            value: tok.tonal,
            ratio: None,
        });
        v.push(Item {
            label: "selected tint".into(),
            value: tok.selected,
            ratio: None,
        });
    }
    if tok.style == Style::Glass {
        v.push(Item {
            label: "ink".into(),
            value: tok.ink,
            ratio: None,
        });
        v.push(Item {
            label: "container".into(),
            value: tok.container,
            ratio: None,
        });
        v.push(Item {
            label: "selected surface".into(),
            value: tok.glass_selected,
            ratio: None,
        });
        v.push(Item {
            label: "tonal".into(),
            value: tok.tonal,
            ratio: None,
        });
    }
    // Pixel art labels an accent-outlined card, not an accent fill.
    let (label_ratio, label_min) = if tok.style == Style::Pixel {
        (contrast(tok.label_on_accent, tok.card), 4.5)
    } else {
        (contrast(tok.label_on_accent, tok.accent), 3.0)
    };
    v.push(Item {
        label: "label on accent".into(),
        value: tok.label_on_accent,
        ratio: Some((label_ratio, label_min)),
    });
    v.push(Item {
        label: "track".into(),
        value: tok.track,
        ratio: None,
    });
    v.push(Item {
        label: "thumb off".into(),
        value: tok.thumb_off,
        ratio: None,
    });
    v.push(Item {
        label: "disabled".into(),
        value: tok.disabled,
        ratio: None,
    });
    v.push(Item {
        label: "expanded".into(),
        value: tok.expanded,
        ratio: None,
    });
    v
}

/// Straight-alpha source-over, to read a see-through token as the eye sees it.
fn over(fg: Rgba, bg: Rgba) -> Rgba {
    let (fa, ba) = (fg.a as f32 / 255.0, bg.a as f32 / 255.0);
    let a = fa + ba * (1.0 - fa);
    if a <= 0.0 {
        return Rgba::TRANSPARENT;
    }
    let ch = |f: u8, b: u8| ((f as f32 * fa + b as f32 * ba * (1.0 - fa)) / a) as u8;
    Rgba {
        r: ch(fg.r, bg.r),
        g: ch(fg.g, bg.g),
        b: ch(fg.b, bg.b),
        a: (a * 255.0) as u8,
    }
}

/// One card or glass surface with an accent pill and two text lines.
fn preview(ui: &mut egui::Ui, tok: &Tokens, ink: Rgba) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 104.0),
        egui::Sense::hover(),
    );
    let p = ui.painter();
    p.rect_filled(rect, 20.0, tok.card.color32());
    p.rect_stroke(
        rect,
        20.0,
        egui::Stroke::new(1.0, tok.outline.color32()),
        egui::StrokeKind::Inside,
    );
    let pill =
        egui::Rect::from_min_size(rect.min + egui::vec2(20.0, 24.0), egui::vec2(112.0, 40.0));
    if tok.style == Style::Pixel {
        // Pixel art: a card-filled box with an accent dot outline and an accent label.
        p.rect_filled(pill, 10.0, tok.card.color32());
        p.rect_stroke(
            pill,
            10.0,
            egui::Stroke::new(2.0, tok.accent.color32()),
            egui::StrokeKind::Inside,
        );
    } else {
        p.rect_filled(pill, 20.0, tok.accent.color32());
    }
    p.text(
        pill.center(),
        egui::Align2::CENTER_CENTER,
        "Accent",
        egui::FontId::proportional(16.0),
        tok.label_on_accent.color32(),
    );
    p.text(
        rect.min + egui::vec2(152.0, 32.0),
        egui::Align2::LEFT_TOP,
        "Primary text",
        egui::FontId::proportional(18.0),
        tok.text.color32(),
    );
    p.text(
        rect.min + egui::vec2(152.0, 58.0),
        egui::Align2::LEFT_TOP,
        "Secondary text",
        egui::FontId::proportional(15.0),
        tok.text2.color32(),
    );
    let line = egui::Rect::from_min_size(
        rect.min + egui::vec2(20.0, 82.0),
        egui::vec2(rect.width() - 40.0, 2.0),
    );
    p.rect_filled(line, 1.0, with_alpha(ink, 0.16).color32());
}

/// A swatch over a two-tone board, so a see-through token shows its alpha.
fn swatch(ui: &mut egui::Ui, c: Rgba, board: Rgba) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(48.0, 22.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 4.0, board.color32());
    let cell = 6.0;
    let mut y = rect.top();
    let mut row = 0;
    while y < rect.bottom() {
        let mut x = rect.left();
        let mut col = 0;
        while x < rect.right() {
            if (row + col) % 2 == 0 {
                let r = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell, cell))
                    .intersect(rect);
                p.rect_filled(r, 0.0, egui::Color32::from_white_alpha(40));
            }
            x += cell;
            col += 1;
        }
        y += cell;
        row += 1;
    }
    p.rect_filled(rect, 4.0, c.color32());
    p.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_white_alpha(48)),
        egui::StrokeKind::Inside,
    );
}
