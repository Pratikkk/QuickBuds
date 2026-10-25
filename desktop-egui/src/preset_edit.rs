//! Edit preset (`PresetActivity`, DESIGN §7.7): the name, a 200 dp preview, the six colour rows with a
//! hex picker under the open one, the contrast warnings, duplicate and delete. The tokens are the phone's
//! `palette_custom` records, so the screen edits the same presets that the Themes page lists.

use crate::glass::{self, white};
use crate::palette::{self, Pal};
use crate::phone;
use crate::{icons, load_settings, save_setting, t, App, Page};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};

const ROW: f32 = 54.0;
const WARN: f32 = 27.5;
const SWATCHES: [&str; 5] = ["#D71920", "#E8742A", "#22A06B", "#3B82F6", "#8B5CF6"];

#[derive(Default)]
pub struct State {
    ready: bool,
    id: String,
    name: String,
    /// The tokens being edited.
    pal: Option<Pal>,
    /// The token row whose picker is open.
    open: Option<usize>,
    hex: String,
    pub ask_delete: bool,
}

#[derive(Clone)]
pub struct Preset { pub id: String, pub name: String, pub pal: Pal }

fn hex(c: Color32) -> String { palette::hex(c) }
fn parse(s: &str) -> Option<Color32> { palette::parse_hex(s) }

pub fn all() -> Vec<Preset> {
    load_settings()["palette_custom"].as_array().map(|a| a.iter().filter_map(|o| {
        let c = |k: &str| o[k].as_str().and_then(parse);
        Some(Preset {
            id: o["id"].as_str()?.to_string(), name: o["name"].as_str()?.to_string(),
            pal: Pal { bg: c("background")?, card: c("card")?, accent: c("accent")?, text: c("text")?, text2: c("textSecondary")?, outline: c("outline")? },
        })
    }).collect()).unwrap_or_default()
}

fn store(list: &[Preset]) {
    let arr: Vec<serde_json::Value> = list.iter().map(|p| serde_json::json!({
        "id": &p.id, "name": &p.name, "background": hex(p.pal.bg), "card": hex(p.pal.card), "accent": hex(p.pal.accent),
        "text": hex(p.pal.text), "textSecondary": hex(p.pal.text2), "outline": hex(p.pal.outline),
    })).collect();
    save_setting("palette_custom", serde_json::json!(arr));
}

fn new_id(list: &[Preset]) -> String {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
    let mut id = format!("custom{n}");
    while list.iter().any(|p| p.id == id) { id.push('1'); }
    id
}

fn default_name(tr: &[String], list: &[Preset]) -> String {
    t(tr, "theme_new_name").replace("%1$d", &(list.len() + 1).to_string())
}

pub fn opened(app: &mut App) {
    let list = all();
    let active = load_settings()["palette_active"].as_str().unwrap_or(palette::DARK).to_string();
    let chosen = list.iter().find(|p| p.id == active).or_else(|| list.first());
    let (id, name, pal) = match chosen {
        Some(p) => (p.id.clone(), p.name.clone(), p.pal),
        None => ("".into(), default_name(&app.tr, &list), palette::built_in(palette::OLED, false)),
    };
    app.preset.id = id;
    app.preset.name = name;
    app.preset.pal = Some(pal);
    app.preset.open = None;
    app.preset.hex = hex(pal.accent);
    app.preset.ready = true;
}

/// The id and name of the preset in the editor.
pub fn title(app: &App) -> String { app.preset.name.clone() }

/// Saves the name and tokens; a new preset joins the list when a slot is free.
fn persist(app: &mut App) -> bool {
    let Some(pal) = app.preset.pal else { return false };
    let mut list = all();
    if let Some(p) = list.iter_mut().find(|p| p.id == app.preset.id) {
        p.name = app.preset.name.trim().to_string();
        p.pal = pal;
        store(&list);
        return true;
    }
    if list.len() >= 3 { return false; }
    let id = new_id(&list);
    list.push(Preset { id: id.clone(), name: app.preset.name.trim().to_string(), pal });
    store(&list);
    app.preset.id = id;
    true
}

/// Copy the editor's preset with the phone's " copy" name. Disabled at three presets.
pub fn duplicate(app: &mut App) {
    if app.preset.pal.is_none() || !persist(app) { return; }
    let mut list = all();
    if list.len() >= 3 { return; }
    let Some(p) = list.iter().find(|p| p.id == app.preset.id).cloned() else { return };
    let id = new_id(&list);
    let name = t(&app.tr, "theme_copy_name").replace("%1$s", &p.name);
    list.push(Preset { id: id.clone(), name: name.clone(), pal: p.pal });
    store(&list);
    app.preset.id = id;
    app.preset.name = name;
}

/// The header's Delete button: ask first.
pub fn ask_delete(app: &mut App) { if app.preset.pal.is_some() { app.preset.ask_delete = true; } }

fn delete(app: &mut App, ctx: &egui::Context) {
    let id = app.preset.id.clone();
    let list: Vec<Preset> = all().into_iter().filter(|p| p.id != id).collect();
    store(&list);
    if load_settings()["palette_active"].as_str() == Some(id.as_str()) { palette::set_active(palette::OLED); }
    app.preset.ready = false;
    app.preset.ask_delete = false;
    app.page = Page::Appearance;
    app.reload_style(ctx);
}

fn lum(c: Color32) -> f64 {
    let (r, g, b) = (c.r() as f64 / 255.0, c.g() as f64 / 255.0, c.b() as f64 / 255.0);
    let ch = |s: f64| if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) };
    0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
}
fn contrast(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (lum(a), lum(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}
fn tokens(p: Pal) -> [(&'static str, &'static str, Color32); 6] {
    [
        ("token_background", "Background", p.bg), ("token_card", "Card", p.card), ("token_accent", "Accent", p.accent),
        ("token_text", "Text", p.text), ("token_text_secondary", "Secondary text", p.text2), ("token_outline", "Outline", p.outline),
    ]
}

fn warning(tr: &[String], p: Pal, i: usize) -> Option<String> {
    let (r, against) = match i {
        2 => (p.accent, p.bg),
        3 => (p.text, p.card),
        4 => (p.text2, p.card),
        _ => return None,
    };
    let min = if i == 2 { 3.0 } else { 4.5 };
    let c = contrast(r, against);
    if c >= min { return None; }
    Some(t(tr, "contrast_warning").replace("%1$s", &format!("{c:.1}")).replace("%2$s", if i == 2 { "Background" } else { "Card" }))
}

fn preview(ui: &Ui, rect: Rect, p: Pal) {
    ui.painter().rect_filled(rect, 16.0, p.bg);
    let card = Rect::from_min_size(Pos2::new(rect.left() + 14.0, rect.top() + 14.0), Vec2::new(rect.width() - 28.0, rect.height() - 28.0));
    ui.painter().rect_filled(card, 12.0, p.card);
    ui.painter().rect_stroke(card, 12.0, Stroke::new(1.0, p.outline), egui::StrokeKind::Inside);
    ui.painter().text(Pos2::new(card.left() + 12.0, card.top() + 16.0), Align2::LEFT_CENTER, "QuickBuds", FontId::proportional(15.0), p.text);
    ui.painter().text(Pos2::new(card.left() + 12.0, card.top() + 34.0), Align2::LEFT_CENTER, "OnePlus Buds 4", FontId::proportional(12.0), p.text2);
    let pill = Rect::from_min_size(Pos2::new(card.left() + 12.0, card.bottom() - 40.0), Vec2::new(card.width() - 24.0, 26.0));
    ui.painter().rect_filled(pill, 13.0, p.accent);
    let ink = if contrast(p.text, p.accent) >= 3.0 { p.text } else { p.bg };
    ui.painter().text(pill.center(), Align2::CENTER_CENTER, "Disconnect", FontId::proportional(12.0), ink);
    ui.painter().circle_filled(Pos2::new(card.right() - 22.0, card.top() + 22.0), 10.0, p.accent);
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    if !app.preset.ready { opened(app); }
    let tr = app.tr.clone();
    let Some(pal) = app.preset.pal else { return body.top() };
    let w = phone::content(body);
    let x0 = body.left();
    let mut y = body.top();

    phone::label(ui, x0, y, t(&tr, "preset_name"), white(0.70));
    y += phone::SECTION;
    let name_rect = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 52.0));
    let field = phone::field_at(ui, name_rect, &mut app.preset.name, 24);
    if field.lost_focus() { persist(app); }
    y += 52.0 + 2.0;

    phone::label(ui, x0, y, t(&tr, "preset_preview"), white(0.70));
    y += phone::SECTION;
    let prev = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 200.0));
    preview(ui, prev, pal);
    y += 200.0;

    phone::label(ui, x0, y, t(&tr, "preset_colors"), white(0.70));
    y += phone::SECTION;

    let mut changed: Option<(usize, Color32)> = None;
    for (i, (key, _label, c)) in tokens(pal).into_iter().enumerate() {
        let warn = warning(&tr, pal, i);
        let mut h = ROW + if warn.is_some() { WARN } else { 0.0 };
        if app.preset.open == Some(i) { h += 56.0; }
        let rect = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW));
        glass::flat_at(ui, rect, 16.0, 1.0);
        let hit = ui.interact(rect, ui.scope_id().with(("tok", i)), Sense::click());
        ui.painter().circle_filled(Pos2::new(rect.left() + 16.0 + 15.0, rect.center().y), 15.0, c);
        ui.painter().circle_stroke(Pos2::new(rect.left() + 16.0 + 15.0, rect.center().y), 15.0, Stroke::new(1.0, white(0.5)));
        glass::text(ui, Pos2::new(rect.left() + 59.4, rect.center().y), Align2::LEFT_CENTER, t(&tr, key), 16.0, 1.0);
        ui.painter().text(Pos2::new(rect.right() - 18.0, rect.center().y), Align2::RIGHT_CENTER, hex(c), FontId::proportional(14.0), white(0.74));
        if hit.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
        if hit.clicked() {
            app.preset.open = if app.preset.open == Some(i) { None } else { Some(i) };
            app.preset.hex = hex(c);
        }
        if app.preset.open == Some(i) {
            let pick = Rect::from_min_size(Pos2::new(rect.left() + 16.0, rect.bottom() + 6.0), Vec2::new(112.0, 43.8));
            let f = glass::text_field_at(ui, pick, &mut app.preset.hex, 7);
            if f.lost_focus() {
                if let Some(v) = parse(&app.preset.hex) { changed = Some((i, v)); }
                app.preset.hex = hex(c);
            }
            let size = 29.7;
            let gap = 8.0;
            let left = rect.right() - 16.0 - (5.0 * size + 4.0 * gap);
            for (n, s) in SWATCHES.iter().enumerate() {
                let Some(v) = parse(s) else { continue };
                let centre = Pos2::new(left + (size + gap) * n as f32 + size / 2.0, pick.center().y);
                let sh = ui.interact(Rect::from_center_size(centre, Vec2::splat(size + 8.0)), ui.scope_id().with(("tsw", i, n)), Sense::click());
                ui.painter().circle_filled(centre, size / 2.0, v);
                if v == c { ui.painter().circle_stroke(centre, size / 2.0 + 2.0, Stroke::new(2.0, white(1.0))); }
                if sh.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
                if sh.clicked() { changed = Some((i, v)); }
            }
        }
        if let Some(text) = warn {
            glass::icon_at(ui, icons::WARNING, Rect::from_center_size(Pos2::new(rect.left() + 16.0 + 7.0, rect.bottom() + 13.0), Vec2::splat(14.0)), white(0.8));
            ui.painter().text(Pos2::new(rect.left() + 16.0 + 22.0, rect.bottom() + 13.0), Align2::LEFT_CENTER, text, FontId::proportional(12.0), white(0.8));
        }
        y += h + 8.0;
    }

    if let Some((i, c)) = changed {
        let Some(mut p) = app.preset.pal else { return y };
        match i { 0 => p.bg = c, 1 => p.card = c, 2 => p.accent = c, 3 => p.text = c, 4 => p.text2 = c, _ => p.outline = c }
        app.preset.pal = Some(p);
        app.preset.hex = hex(c);
        persist(app);
    }

    if app.preset.ask_delete {
        let mut decision = None;
        let active = palette::active_id(crate::palette::is_light_system(ui.ctx())) == app.preset.id;
        let body = if active { t(&tr, "preset_delete_active") } else { "" };
        let title = t(&tr, "preset_delete_title").replace("%1$s", &app.preset.name);
        let open = crate::settings::sheets::sheet(ui.ctx(), "preset-delete", &title, crate::settings::sheets::Title::Centre, Vec2::new(crate::settings::sheets::WIDTH, 243.2), |ui, inner| {
            let msg = Rect::from_min_max(inner.min + Vec2::new(17.0, 4.0), Pos2::new(inner.right(), inner.bottom() - 66.0));
            phone::paragraph(ui, msg, body, 14.0, white(0.80), true);
            decision = crate::settings::sheets::actions(ui, inner, t(&tr, "dialog_cancel"), t(&tr, "preset_delete"));
        });
        match decision {
            Some(true) => delete(app, ui.ctx()),
            Some(false) => app.preset.ask_delete = false,
            None if !open => app.preset.ask_delete = false,
            None => {}
        }
    }
    y + phone::GAP
}
