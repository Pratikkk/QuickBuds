//! The Earbud settings page (`EarbudSettingsActivity`): the feature switches the buds list, the sounds and the firmware.
//! Personalized ANC and the fit test (their flows) are not ported yet.

use crate::glass::{self, Choice, Trailing};
use crate::home_logic::{self, ROWS};
use crate::session::{Cmd, Status};
use crate::types::*;
use crate::{icons, t, App, Confirm};
use eframe::egui::{self, Align2, Pos2, Rect, Ui, Vec2};
use std::time::{Duration, Instant};

const ROW: f32 = 72.0;
const MAX_W: f32 = 760.0;

#[derive(Default)]
pub struct State {
    /// The picker dialog open: (kind 1 game sound, 2 head gesture).
    picking: Option<u8>,
    alert: Option<f32>,
    tap: Option<f32>,
    // Find my earbuds: the warning before a loud tone, and the tone playing.
    find_open: bool,
    find_warn: bool,
    find_playing: bool,
    // Fit test: 0 ready, 1 insert the buds, 2 running, 3 failed, 4 perfect, 5 adjust.
    fit_open: bool,
    fit_state: u8,
    fit_started: bool,
    fit_shown: bool,
    fit_deadline: Option<Instant>,
    fit_seq: u32,
    // Personalized ANC: 0 asking, 1 stored result offered, 2 test intro, 3 testing, 4 failed, 5 wear both buds, 6 done.
    pnc_open: bool,
    pnc_state: u8,
    pnc_reason: u8,
    pnc_deadline: Option<Instant>,
    pnc_seq: u32,
}

/// HeyMelody's names for the game sound types; a type without one is not offered.
fn game_label(tr: &[String], v: u8) -> Option<String> {
    Some(t(tr, match v { 0 => "anc_seg_off", 1 => "game_sound_type_peace", 3 => "game_sound_type_shooter", _ => return None }).into())
}

fn head_label(tr: &[String], v: u8) -> Option<String> {
    Some(t(tr, match v { 0 => "head_motion_nod", 1 => "head_motion_shake", _ => return None }).into())
}

fn label(ui: &Ui, x: f32, y: f32, s: &str) {
    glass::text(ui, Pos2::new(x + 6.0, y + 14.0), Align2::LEFT_CENTER, s, 14.0, 0.72);
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    let s = app.snap.clone();
    let tr = app.tr.clone();
    // Wide window: the model, the switches and the firmware on the left; sounds, fit test and find on the right.
    let two = body.width() >= 900.0;
    let w = if two { ((body.width() - 14.0) / 2.0).floor() } else { body.width().min(MAX_W) };
    let x0 = body.left();
    let xr = if two { x0 + w + 14.0 } else { x0 };
    let mut y = body.top();
    let mut yr = body.top();
    let on = s.status == Status::On;
    let flagged = |flag: Option<&str>| s.manual && flag.is_some_and(|f| s.model.is_some_and(|m| m[f] == 1));

    // The model row: Automatic with what was detected, or the pick.
    {
        let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW));
        glass::flat_at(ui, card, 28.0, 1.0);
        let (clicked, _) = glass::row_at(ui, "model", card, icons::EARBUD, t(&tr, "model_title"), &crate::models::summary(app), Trailing::Chevron, true);
        if clicked { app.page = crate::Page::Models; }
        y += ROW + 18.0;
    }

    // Feature switches, in the phone's order.
    struct Item { id: u8, icon: &'static str, title: &'static str, sub: &'static str, on: bool, confirm: Option<&'static str>, confirm_off: bool, kind: u8, choice: Option<String> }
    let mut items: Vec<Item> = vec![];
    for &(id, flag, icon, title, sub, confirm, confirm_off) in ROWS {
        let v = match s.features.iter().find(|f| f.0 == id) { Some(f) => f.1, None if flagged(flag) => 0, None => continue };
        // The picker under its switch: which game sound effect, or which head gesture answers.
        let (kind, choice) = match id {
            0x27 if s.manual || s.caps.supports(CMD_GAME_SOUND) => (1, s.game_sound.as_ref().and_then(|g| game_label(&tr, g.0))),
            0x3B if s.caps.supports(CMD_SET_HEAD_MOTION) => (2, s.head_motion.and_then(|v| head_label(&tr, v))),
            _ => (0, None),
        };
        items.push(Item { id, icon, title, sub, on: v == 1, confirm, confirm_off, kind, choice });
    }
    // Personalized ANC first, as on the phone: the model's flag too. Turning it on runs the test flow, not a write.
    let pnc_listed = s.features.iter().find(|f| f.0 == FEATURE_PERSONAL_NOISE).map(|f| f.1);
    let pnc = s.model.map_or(true, |m| m["personalAnc"] == 1)
        && (pnc_listed.is_some() || flagged(Some("personalAnc")))
        && (s.caps.supports(CMD_PERSONAL_NOISE) || s.manual);
    if pnc || !items.is_empty() {
        if !two { label(ui, x0, y, t(&tr, "earbuds_section_features")); y += 34.0; }
        let pnc_h = if pnc { ROW } else { 0.0 };
        let rows_h: f32 = pnc_h + items.iter().map(|i| ROW + if i.kind > 0 && i.on { 56.0 } else { 0.0 }).sum::<f32>();
        let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, rows_h));
        glass::flat_at(ui, card, 28.0, 1.0);
        let mut cy = y;
        if pnc {
            let r = Rect::from_min_size(Pos2::new(x0, cy), Vec2::new(w, ROW));
            let pon = pnc_listed == Some(1);
            let (_, toggled) = glass::row_at(ui, "pnc", r, icons::MODE_ANC_MEDIUM, t(&tr, "pnc_title"), t(&tr, "pnc_sub"), Trailing::Switch(pon), on);
            if toggled {
                if pon { app.send(Cmd::Features(vec![(FEATURE_PERSONAL_NOISE, false)])); } else { pnc_start(app); }
            }
            cy += ROW;
        }
        for (n, i) in items.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(x0, cy), Vec2::new(w, ROW));
            if n > 0 || pnc { glass::divider_at(ui, x0, x0 + w, r.top()); }
            let (_, toggled) = glass::row_at(ui, ("feat", i.id), r, i.icon, t(&tr, i.title), t(&tr, i.sub), Trailing::Switch(i.on), on);
            if toggled {
                let cmds = home_logic::feature_cmds(&s, i.id, !i.on);
                // Asks when turning on; turning off too where the row says so.
                match i.confirm.filter(|_| !i.on || i.confirm_off) {
                    Some(msg) => app.confirm = Some(Confirm { cmds, title: i.title, msg, accept: "codec_dialog_accept" }),
                    None => for c in cmds { app.send(c); },
                }
            }
            cy += ROW;
            if i.kind > 0 && i.on {
                let pr = Rect::from_min_size(Pos2::new(x0, cy), Vec2::new(w, 56.0));
                let title = t(&tr, if i.kind == 1 { "game_sound_type_title" } else { "head_motion_type_title" });
                let (clicked, _) = glass::row_at(ui, ("pick", i.id), pr, icons::CHEVRON_DOWN, title, "", Trailing::Value(i.choice.clone().unwrap_or("—".into())), on);
                if clicked { app.earbuds.picking = Some(i.kind); }
                cy += 56.0;
            }
        }
        y += rows_h + 18.0;
    }

    {
    let x0 = xr;
    if two { std::mem::swap(&mut y, &mut yr); }
    // Sounds: the alert volume, the tap sensitivity.
    let has_alert = s.caps.supports(CMD_SET_ALERT_VOLUME) && s.alert_volume.is_some();
    let has_tap = s.model.is_some_and(|m| m["tapLevel"] == 1) && (s.manual || s.caps.supports(CMD_SET_TAP_LEVEL) && s.tap_level.is_some());
    if has_alert || has_tap {
        if !two { label(ui, x0, y, t(&tr, "earbuds_section_sounds")); y += 34.0; }
        let n = has_alert as usize + has_tap as usize;
        let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 96.0 * n as f32));
        glass::flat_at(ui, card, 28.0, 1.0);
        let mut cy = y;
        if has_alert {
            let mut v = app.earbuds.alert.unwrap_or(s.alert_volume.unwrap_or(5) as f32);
            glass::text(ui, Pos2::new(x0 + 24.0, cy + 26.0), Align2::LEFT_CENTER, t(&tr, "row_alert_title"), 15.0, 1.0);
            glass::icon_at(ui, if v <= 1.0 { icons::VOLUME_OFF } else { icons::VOLUME }, Rect::from_center_size(Pos2::new(x0 + 36.0, cy + 66.0), Vec2::splat(22.0)), glass::white(1.0));
            let sr = Rect::from_min_size(Pos2::new(x0 + 60.0, cy + 46.0), Vec2::new(w - 90.0, 40.0));
            let sl = glass::slider_at(ui, "alert", sr, &mut v, 1.0, 10.0, true);
            if sl.changed { app.earbuds.alert = Some(v); }
            if sl.released { app.send(Cmd::AlertVolume(v as u8)); app.earbuds.alert = None; }
            cy += 96.0;
        }
        if has_tap {
            let mut v = app.earbuds.tap.unwrap_or(s.tap_level.map_or(3, |t| t.0) as f32);
            glass::text(ui, Pos2::new(x0 + 24.0, cy + 26.0), Align2::LEFT_CENTER, t(&tr, "tap_level_title"), 15.0, 1.0);
            let sr = Rect::from_min_size(Pos2::new(x0 + 24.0, cy + 46.0), Vec2::new(w - 90.0, 40.0));
            let sl = glass::slider_at(ui, "tap", sr, &mut v, 1.0, 5.0, true);
            glass::text(ui, Pos2::new(sr.right() + 16.0, sr.center().y), Align2::LEFT_CENTER, &format!("{}", v as i32), 15.0, 1.0);
            if sl.changed { app.earbuds.tap = Some(v); }
            if sl.released { app.send(Cmd::TapLevel(v as u8)); app.earbuds.tap = None; }
        }
        y += card.height() + 18.0;
    }

    // Fit test and Find my earbuds: each opens its dialog.
    let mut has_rows = vec![];
    if s.caps.supports(CMD_FIT_TEST) { has_rows.push(0); }
    if s.caps.supports(CMD_FIND_BUDS) { has_rows.push(1); }
    if !has_rows.is_empty() {
        let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW * has_rows.len() as f32));
        glass::flat_at(ui, card, 28.0, 1.0);
        for (n, kind) in has_rows.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(x0, y + ROW * n as f32), Vec2::new(w, ROW));
            if n > 0 { glass::divider_at(ui, x0, x0 + w, r.top()); }
            let (clicked, _) = if *kind == 0 {
                glass::row_at(ui, "fit", r, icons::EARBUD, t(&tr, "fit_title"), t(&tr, "fit_sub"), Trailing::Chevron, on)
            } else {
                glass::row_at(ui, "find", r, icons::FIND_BUDS, t(&tr, "row_find_title"), t(&tr, "row_find_sub"), Trailing::Chevron, on)
            };
            if clicked { if *kind == 0 { app.earbuds.fit_open = true; app.earbuds.fit_state = 0; app.earbuds.fit_shown = false; app.earbuds.fit_seq = s.fit.2; } else { app.earbuds.find_open = true; app.earbuds.find_warn = false; } }
        }
        y += card.height() + 18.0;
    }

    if two { std::mem::swap(&mut y, &mut yr); }
    }

    // About: the firmware. No updates here: a failed flash can brick the buds, so it points to HeyMelody.
    if s.caps.supports(CMD_QUERY_FIRMWARE) {
        if !two { label(ui, x0, y, t(&tr, "earbuds_section_about")); y += 34.0; }
        let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW));
        glass::flat_at(ui, card, 28.0, 1.0);
        glass::row_at(ui, "fw", card, icons::INFO, t(&tr, "row_firmware_title"), s.firmware.as_deref().unwrap_or("—"), Trailing::None, true);
        y += ROW + 18.0;
    }

    // The picker dialog.
    if let Some(kind) = app.earbuds.picking {
        let (title, list): (&str, Vec<(u8, String)>) = if kind == 1 {
            let offered = s.game_sound.as_ref().map(|g| g.1.clone()).filter(|v| !v.is_empty()).unwrap_or(vec![3]);
            let mut game: Vec<u8> = vec![0];
            for v in offered { if !game.contains(&v) { game.push(v); } }
            ("game_sound_type_title", game.into_iter().filter_map(|v| Some((v, game_label(&tr, v)?))).collect())
        } else {
            ("head_motion_type_title", [0u8, 1].into_iter().filter_map(|v| Some((v, head_label(&tr, v)?))).collect())
        };
        let current = if kind == 1 { s.game_sound.as_ref().map(|g| g.0) } else { s.head_motion };
        let options: Vec<String> = list.iter().map(|(_, l)| l.clone()).collect();
        let cur = list.iter().position(|(v, _)| Some(*v) == current).unwrap_or(usize::MAX);
        match glass::choice_dialog(ui.ctx(), "earbuds-pick", t(&tr, title), &options, cur) {
            Choice::Pick(i) => {
                let v = list[i].0;
                app.send(if kind == 1 { Cmd::GameSoundType(v) } else { Cmd::HeadMotion(v) });
                app.earbuds.picking = None;
            }
            Choice::Close => app.earbuds.picking = None,
            Choice::Open => {}
        }
    }
    dialogs(app, ui);
    y.max(yr)
}

/// (left, right) in an ear. Without an "in ear" seen, "out" may only mean the model has no sensor: counted as in.
fn worn(app: &App) -> (bool, bool) {
    let blind = !crate::overview::wear_sensor_seen(&app.snap, app.demo);
    let f = |w: u8| matches!(w, 3 | 7) || blind && matches!(w, 1 | 5);
    (f(app.snap.wear[0]), f(app.snap.wear[1]))
}

fn pnc_start(app: &mut App) {
    let (l, r) = worn(app);
    app.earbuds.pnc_open = true;
    app.earbuds.pnc_seq = app.snap.pnc.2;
    if !(l && r) { app.earbuds.pnc_state = 5; return; }
    app.earbuds.pnc_state = 0;
    app.earbuds.pnc_deadline = Some(Instant::now() + Duration::from_secs(5));
    app.send(Cmd::PncQuery);
}

fn pnc_test(app: &mut App) {
    let (l, r) = worn(app);
    if !(l && r) { app.earbuds.pnc_state = 5; return; }
    app.earbuds.pnc_state = 3;
    app.earbuds.pnc_deadline = Some(Instant::now() + Duration::from_secs(15));
    app.send(Cmd::Pnc(1));
}

fn pnc_finish(app: &mut App) {
    if app.earbuds.pnc_state == 3 { app.send(Cmd::Pnc(3)); }
    app.earbuds.pnc_state = 0;
    app.earbuds.pnc_deadline = None;
    app.earbuds.pnc_open = false;
}

fn fit_play(app: &mut App) {
    let (l, r) = worn(app);
    if !(l && r) { app.earbuds.fit_state = 1; return; }
    app.earbuds.fit_state = 2;
    app.earbuds.fit_started = true;
    app.earbuds.fit_shown = false;
    app.earbuds.fit_seq = app.snap.fit.2;
    app.earbuds.fit_deadline = Some(Instant::now() + Duration::from_secs(15));
    app.send(Cmd::FitTest(true));
}

fn fit_stop(app: &mut App) {
    if app.earbuds.fit_started { app.earbuds.fit_started = false; app.send(Cmd::FitTest(false)); }
    app.earbuds.fit_deadline = None;
}

/// Every frame: the results the buds sent and the timeouts of the fit test and Personalized ANC.
fn tick(app: &mut App) {
    let now = Instant::now();
    let s = app.snap.clone();
    // Fit test
    if app.earbuds.fit_open && app.earbuds.fit_state == 2 {
        if s.fit.2 != app.earbuds.fit_seq {
            app.earbuds.fit_seq = s.fit.2;
            let ok = |v: u8| matches!(v, 0 | 1 | 6);
            if !ok(s.fit.0) || !ok(s.fit.1) { app.earbuds.fit_state = 3; }
            else { app.earbuds.fit_shown = true; app.earbuds.fit_state = if s.fit.0 == 1 && s.fit.1 == 1 { 4 } else { 5 }; }
        }
        if app.earbuds.fit_deadline.is_some_and(|d| now >= d) { app.earbuds.fit_state = 3; }
    }
    // Personalized ANC
    if app.earbuds.pnc_open {
        if s.pnc.2 != app.earbuds.pnc_seq {
            app.earbuds.pnc_seq = s.pnc.2;
            let (kind, value) = (s.pnc.0, s.pnc.1);
            let st = app.earbuds.pnc_state;
            if kind == 1 && st == 0 { app.earbuds.pnc_state = if value != 0 { 1 } else { 2 }; app.earbuds.pnc_deadline = None; }
            // A rejected write fails the test as well (status 15: another device busy).
            if kind == 2 && st == 3 && value != 0 { app.earbuds.pnc_reason = 0; app.earbuds.pnc_state = 4; }
            if kind == 3 && st == 3 {
                if value == 0 { app.earbuds.pnc_state = 6; app.send(Cmd::StatusRead); } else { app.earbuds.pnc_reason = value; app.earbuds.pnc_state = 4; }
            }
        }
        if app.earbuds.pnc_deadline.is_some_and(|d| now >= d) {
            app.earbuds.pnc_deadline = None;
            match app.earbuds.pnc_state { 0 => app.earbuds.pnc_state = 2, 3 => { app.earbuds.pnc_reason = 0; app.earbuds.pnc_state = 4; } _ => {} }
        }
    }
}

/// A dialog with a title, a body and the buttons: (left label, primary label or "" for none).
fn flow_dialog(ui: &mut Ui, id: &str, title: &str, body: &str, extra: Option<&str>, left: &str, primary: &str) -> Option<bool> {
    let mut out = None;
    // No auto-close: only the buttons end a flow (a click outside would otherwise start a test).
    glass::dialog(ui.ctx(), id, title, Vec2::new(440.0, 280.0), |ui| {
        ui.add(egui::Label::new(egui::RichText::new(body).size(14.0).color(glass::white(0.8))).wrap());
        if let Some(x) = extra {
            let r = ui.max_rect();
            glass::text(ui, Pos2::new(r.center().x, r.top() + 110.0), Align2::CENTER_CENTER, x, 15.0, 1.0);
        }
        out = glass::dialog_buttons(ui, left, primary);
    });
    out
}

fn dialogs(app: &mut App, ui: &mut Ui) {
    tick(app);
    if app.earbuds.fit_open || app.earbuds.pnc_open { ui.ctx().request_repaint_after(Duration::from_millis(250)); }
    let tr = app.tr.clone();
    let s = app.snap.clone();

    // Find my earbuds
    if app.earbuds.find_open {
        let warn = app.earbuds.find_warn;
        let playing = app.earbuds.find_playing;
        let primary = t(&tr, if playing { "find_stop" } else if warn { "find_warn_play" } else { "find_play" });
        let out = flow_dialog(ui, "find", t(&tr, if warn { "find_warn_title" } else { "find_title" }), t(&tr, if warn { "find_warn_msg" } else { "find_hint" }), None, t(&tr, "dialog_close"), primary);
        match out {
            Some(false) => { app.earbuds.find_warn = false; app.earbuds.find_open = false; if playing { app.send(Cmd::Find(false)); app.earbuds.find_playing = false; } }
            Some(true) => {
                if playing { app.send(Cmd::Find(false)); app.earbuds.find_playing = false; }
                else if !warn && { let (l, r) = worn(app); l || r } { app.earbuds.find_warn = true; }
                else { app.earbuds.find_warn = false; app.earbuds.find_playing = true; app.send(Cmd::Find(true)); }
            }
            None => {}
        }
    }

    // Fit test
    if app.earbuds.fit_open {
        let state = app.earbuds.fit_state as usize;
        let title = ["fit_title", "fit_insert", "fit_keep", "fit_failed", "fit_perfect", "fit_adjust"][state.min(5)];
        let body = if state == 2 || state == 4 { String::new() } else if state == 5 {
            let who = if s.fit.0 != 1 && s.fit.1 != 1 { "fit_both" } else if s.fit.0 != 1 { "fit_left" } else { "fit_right" };
            t(&tr, "fit_adjust_tips").replace("%1$s", t(&tr, who))
        } else { t(&tr, "fit_hint").to_string() };
        let word = |v: u8| match v { 1 => t(&tr, "fit_good"), 0 => t(&tr, "fit_average"), 6 => t(&tr, "fit_poor"), _ => "" };
        let extra = app.earbuds.fit_shown.then(|| format!("{} · {}     {} · {}", t(&tr, "status_left"), word(s.fit.0), t(&tr, "status_right"), word(s.fit.1)));
        let primary = t(&tr, match state { 2 => "fit_playing", 4 => "gesture_done", 5 => "fit_again", _ => "fit_play" });
        match flow_dialog(ui, "fit", t(&tr, title), &body, extra.as_deref(), t(&tr, "dialog_close"), primary) {
            Some(false) => { fit_stop(app); app.earbuds.fit_state = 0; app.earbuds.fit_shown = false; app.earbuds.fit_open = false; }
            Some(true) => {
                if state == 4 { fit_stop(app); app.earbuds.fit_state = 0; app.earbuds.fit_shown = false; app.earbuds.fit_open = false; }
                else if state != 2 { fit_play(app); }
            }
            None => {}
        }
    }

    // Personalized ANC
    if app.earbuds.pnc_open {
        let st = app.earbuds.pnc_state as usize;
        let title = ["pnc_title", "pnc_stored_title", "pnc_test_title", "pnc_title", "pnc_failed", "pnc_title", "pnc_title"][st.min(6)];
        let ri = if app.earbuds.pnc_reason <= 5 { app.earbuds.pnc_reason as usize } else { 0 };
        let reason = ["", "pnc_fail_quiet", "pnc_fail_fit", "pnc_fail_wind", "pnc_fail_still", "pnc_fail_audio"][ri];
        let body = match st {
            0 => t(&tr, "conn_connecting"),
            1 => t(&tr, "pnc_stored_body"),
            2 => t(&tr, "pnc_test_body"),
            3 => t(&tr, "pnc_testing"),
            4 => if reason.is_empty() { "" } else { t(&tr, reason) },
            5 => t(&tr, "pnc_wear"),
            _ => t(&tr, "pnc_done"),
        };
        let left = t(&tr, if st == 1 { "pnc_test_again" } else if st == 6 { "dialog_close" } else { "dialog_cancel" });
        let primary = if matches!(st, 1 | 2 | 4 | 5) { t(&tr, match st { 1 => "pnc_use", 2 => "pnc_start", _ => "pnc_retry" }) } else { "" };
        match flow_dialog(ui, "pnc", t(&tr, title), body, None, left, primary) {
            Some(false) => if st == 1 { pnc_test(app); } else { pnc_finish(app); },
            Some(true) => match st {
                1 => { app.send(Cmd::Pnc(2)); app.earbuds.pnc_state = 0; app.earbuds.pnc_open = false; }
                5 => pnc_start(app),
                _ => pnc_test(app),
            },
            None => {}
        }
    }
}
