//! The sheets that the phone opens from Home and Earbud settings: connection, wear, firmware, crash and
//! the launch prompts. The floating plate is `settings::sheets`, 34 dp, 363.4 dp wide and scrim 0.45.

use crate::glass::{self, white};
use crate::phone;
use crate::settings::sheets::{self, Title, WIDTH};
use crate::{load_settings, save_setting, t, App};
use eframe::egui::{self, FontId, Pos2, Rect, ScrollArea, Ui, Vec2};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Prompt { #[default] None, Reply, Test, Issues }

#[derive(Default)]
pub struct State {
    pub wear: bool,
    pub firmware: bool,
    pub crash: bool,
    pub prompt: Prompt,
    /// The launch prompts were checked for this Home visit.
    checked: bool,
    /// The crash report's timestamp, remembered when the sheet closes.
    crash_stamp: u64,
}

/// The sheets that a screenshot key opens (`QB_SHEET=wear|firmware|crash|reply|test|issues`).
pub fn debug_open(app: &mut App, key: &str) {
    match key {
        "wear" => app.sheets.wear = true,
        "connection" => app.disconnect_ask = true,
        "add" => app.dual.add = true,
        "feedback-preview" => { app.feedback.preview = true; app.feedback.ready = true; app.feedback.model = app.snap.name.clone(); }
        "preset-delete" => app.preset.ask_delete = true,
        "test-preview" => { app.test.preview = true; app.test.started = true; }
        "firmware" => app.sheets.firmware = true,
        "crash" => { app.sheets.crash = true; app.sheets.crash_stamp = 1; }
        "reply" => app.sheets.prompt = Prompt::Reply,
        "test" => app.sheets.prompt = Prompt::Test,
        "issues" => app.sheets.prompt = Prompt::Issues,
        _ => {}
    }
}

fn body_lines(ui: &mut Ui, rect: Rect, text: &str, size: f32) {
    let galley = ui.painter().layout(text.to_string(), FontId::proportional(size), white(0.80), rect.width());
    ui.painter().galley(rect.min, galley, white(0.80));
}

/// Wear detection: auto play/pause and smart auto-pause. Turning one on turns the other off.
fn wear(app: &mut App, ui: &mut Ui) {
    let tr = app.tr.clone();
    let mut auto = load_settings()["wear_auto"].as_bool().unwrap_or(true);
    let mut smart = load_settings()["smart_auto_pause"].as_bool().unwrap_or(false);
    let labels = [t(&tr, "wear_firmware_title"), t(&tr, "wear_smart_title")];
    let subs = [t(&tr, "wear_firmware_sub"), t(&tr, "wear_smart_sub")];
    let inner_w = WIDTH - 28.0;
    let h0 = sheets::row_height(ui, labels[0], subs[0], inner_w);
    let h1 = sheets::row_height(ui, labels[1], subs[1], inner_w);
    let height = 74.3 + h0 + 8.0 + h1 + 16.0;
    let mut changed = false;
    let open = sheets::sheet(ui.ctx(), "wear-sheet", t(&tr, "wear_title"), Title::Centre, Vec2::new(WIDTH, height), |ui, inner| {
        let r0 = Rect::from_min_size(inner.min, Vec2::new(inner.width(), h0));
        if sheets::row_at(ui, "wear-auto", r0, labels[0], subs[0], auto) { auto = !auto; smart = false; changed = true; }
        let r1 = Rect::from_min_size(Pos2::new(inner.left(), r0.bottom() + 8.0), Vec2::new(inner.width(), h1));
        if sheets::row_at(ui, "wear-smart", r1, labels[1], subs[1], smart) { smart = !smart; auto = false; changed = true; }
    });
    if changed {
        save_setting("wear_auto", auto.into());
        save_setting("smart_auto_pause", smart.into());
    }
    if !open { app.sheets.wear = false; }
}

/// Firmware: no update from this app.
fn firmware(app: &mut App, ui: &mut Ui) {
    let tr = app.tr.clone();
    let body = t(&tr, "firmware_dialog_body");
    let h = 74.3 + phone::wrap_height(ui, body, 14.0, WIDTH - 28.0) + 16.0 + 50.0 + 16.0;
    let mut decision = None;
    let open = sheets::sheet(ui.ctx(), "firmware-sheet", t(&tr, "firmware_dialog_title"), Title::Centre, Vec2::new(WIDTH, h), |ui, inner| {
        let msg = Rect::from_min_max(inner.min + Vec2::new(14.0, 4.0), Pos2::new(inner.right() - 14.0, inner.bottom() - 66.0));
        phone::paragraph(ui, msg, body, 14.0, white(0.80), true);
        decision = sheets::actions(ui, inner, "", t(&tr, "dialog_close"));
    });
    if decision.is_some() || !open { app.sheets.firmware = false; }
}

/// The crash report: saved text, Send and Share.
fn crash(app: &mut App, ui: &mut Ui) {
    let tr = app.tr.clone();
    let text = crash_text();
    let mut decision = None;
    let open = sheets::sheet(ui.ctx(), "crash-sheet", t(&tr, "crash_title"), Title::Centre, Vec2::new(WIDTH, 420.0), |ui, inner| {
        let msg = Rect::from_min_size(inner.min + Vec2::new(14.0, 0.0), Vec2::new(inner.width() - 28.0, 34.0));
        body_lines(ui, msg, t(&tr, "crash_saved_to"), 14.0);
        let log = Rect::from_min_max(Pos2::new(inner.left() + 14.0, msg.bottom() + 6.0), Pos2::new(inner.right() - 14.0, inner.bottom() - 66.0));
        glass::flat_at(ui, log, 12.0, 1.0);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(log.shrink(10.0)));
        child.set_clip_rect(log.shrink(10.0));
        ScrollArea::vertical().max_height(log.height() - 20.0).show(&mut child, |ui| {
            ui.add(egui::Label::new(egui::RichText::new(&text).monospace().size(12.0).color(white(0.80))));
        });
        decision = sheets::actions(ui, inner, t(&tr, "crash_share"), t(&tr, "problem_send"));
    });
    if decision.is_some() || !open {
        save_setting("crash_shown", app.sheets.crash_stamp.into());
        app.sheets.crash = false;
    }
}

fn crash_path() -> Option<std::path::PathBuf> {
    crate::settings_path().and_then(|p| p.parent().map(|d| d.join("crash.txt")))
}

fn crash_stamp() -> u64 {
    crash_path().and_then(|p| std::fs::metadata(p).ok()).and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs())
}

fn crash_text() -> String {
    crash_path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_else(|| {
        "QuickBuds 5.2.0 (egui)\npanicked at desktop-egui/src/main.rs\n\nThis is a demo crash report for the sheet.".into()
    })
}

fn prompt(app: &mut App, ui: &mut Ui) {
    let tr = app.tr.clone();
    let (title, body, cancel, accept) = match app.sheets.prompt {
        Prompt::Reply => (t(&tr, "reply_title"), t(&tr, "about_privacy_reply"), "", t(&tr, "welcome_done")),
        Prompt::Test => (t(&tr, "report_title"), t(&tr, "test_ask_body"), t(&tr, "test_ask_later"), t(&tr, "test_title")),
        Prompt::Issues => (t(&tr, "settings_issues_title"), t(&tr, "issues_dialog_body"), t(&tr, "test_ask_later"), t(&tr, "issues_dialog_action")),
        Prompt::None => return,
    };
    let h = 74.3 + phone::wrap_height(ui, body, 14.0, WIDTH - 28.0) + 16.0 + 50.0 + 16.0;
    let mut decision = None;
    let open = sheets::sheet(ui.ctx(), "prompt-sheet", title, Title::Centre, Vec2::new(WIDTH, h), |ui, inner| {
        let msg = Rect::from_min_max(inner.min + Vec2::new(14.0, 4.0), Pos2::new(inner.right() - 14.0, inner.bottom() - 66.0));
        phone::paragraph(ui, msg, body, 14.0, white(0.80), true);
        decision = sheets::actions(ui, inner, cancel, accept);
    });
    if decision.is_some() || !open {
        match app.sheets.prompt {
            Prompt::Reply => save_setting("reply_shown", true.into()),
            Prompt::Test => save_setting("test_asked", app.snap.name.clone().into()),
            Prompt::Issues => save_setting("issues_announced", app.snap.name.clone().into()),
            Prompt::None => {}
        }
        app.sheets.prompt = Prompt::None;
    }
}

/// The launch prompts of DESIGN §7.1, one at a time, in order. A Home visit checks once.
fn start(app: &mut App) {
    if app.demo || app.page != crate::Page::Overview || app.sheets.checked { return; }
    app.sheets.checked = true;
    let stamp = crash_stamp();
    if stamp != 0 && load_settings()["crash_shown"].as_u64() != Some(stamp) {
        app.sheets.crash_stamp = stamp;
        app.sheets.crash = true;
        return;
    }
    if !load_settings()["reply_shown"].as_bool().unwrap_or(false) { app.sheets.prompt = Prompt::Reply; return; }
    if load_settings()["test_asked"].as_str() != Some(app.snap.name.as_str()) { app.sheets.prompt = Prompt::Test; return; }
    if load_settings()["issues_announced"].as_str() != Some(app.snap.name.as_str()) { app.sheets.prompt = Prompt::Issues; }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    start(app);
    if app.sheets.prompt != Prompt::None { prompt(app, ui); }
    if app.sheets.wear { wear(app, ui); }
    if app.sheets.firmware { firmware(app, ui); }
    if app.sheets.crash { crash(app, ui); }
    if app.page != crate::Page::Overview { app.sheets.checked = false; }
}
