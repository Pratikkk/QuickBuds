//! Hearing profile (`GoldenSoundActivity`, `GoldenTestSheet`): the switch, the profiles kept on this PC (a click
//! applies one, the one on the buds is lit), the radar of the active one, and the hearing test.
//! The buds hold only the active record; it is read when the page opens and added to the list.
//! Timers of the test are deadlines checked every frame (`tick`).

use crate::glass::{self, white, Trailing};
use crate::hearing_logic::*;
use crate::session::{Cmd, Status, HEARING};
use crate::types::*;
use crate::{cmd, icons, load_settings, save_setting, t, App};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Shape, Stroke, Ui, Vec2};
use serde_json::json;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const ROW: f32 = 64.0;
const MAX_W: f32 = 760.0;

#[derive(Clone, Copy, PartialEq, Default)]
enum Step { #[default] Idle, Scan, Test, Finish, Done }

/// What the test dialog's main button does.
#[derive(Clone, Copy, Default, PartialEq)]
enum Button { #[default] Start, Next, Again, None }

#[derive(Default)]
pub struct State {
    active: u32,
    hearing: HashMap<u32, (Vec<f32>, Vec<f32>)>,
    scan: HashMap<u32, Curves>,
    ear: usize,
    // The test
    open: bool,
    step: Step,
    button: Button,
    title: String,
    message: String,
    slider: bool,
    done: bool,
    uid: u32,
    index: usize,
    values: [i8; 12],
    scan_data: Vec<u8>,
    warned: bool,
    stop: f32,
    last_tone: Option<Instant>,
    toned: Option<usize>,
    /// Deadlines: the scan or the filter wait, a delayed tone, the delayed curve request.
    timeout: Option<Instant>,
    tone_at: Option<Instant>,
    curves_at: Option<Instant>,
    delete: Option<usize>,
}

fn send(app: &App, frames: Vec<Frame>) { app.send(Cmd::Hearing(frames)); }

/// The page opened: the record on the buds, so one made in HeyMelody or on the phone shows up too.
pub fn opened(app: &App) {
    send(app, vec![(CMD_HEARING_ACTIVE, vec![]), (CMD_HEARING_ACTIVE_SCAN, vec![])]);
}

/// The test dialog's texts; `message` None keeps the current one.
fn show_text(app: &mut App, title: &str, message: Option<&str>, button: Button, slider: bool) {
    let tr = app.tr.clone();
    let h = &mut app.hearing;
    h.title = t(&tr, title).into();
    if let Some(m) = message { h.message = if m.is_empty() { String::new() } else { t(&tr, m).into() }; }
    h.button = button;
    h.done = false;
    h.slider = slider;
}

fn start(app: &mut App) {
    let w = app.snap.wear;
    let blind = !crate::overview::wear_sensor_seen(&app.snap, app.demo);
    if ![w[0], w[1]].iter().all(|x| matches!(x, 3 | 7) || blind && matches!(x, 1 | 5)) {
        app.hearing.title = t(&app.tr, "fit_insert").into();
        return;
    }
    let uid = (random() % 0x7FFF_FFFE) as u32 + 1;
    app.hearing.uid = uid;
    app.hearing.scan_data.clear();
    app.hearing.warned = false;
    // Ear scan only where HeyMelody's model list has it (`earScan`).
    let scan = app.snap.model.is_some_and(|m| m["hearingScan"] == 1) && app.snap.caps.supports(CMD_HEARING_SCAN_DATA);
    if scan {
        app.hearing.step = Step::Scan;
        show_text(app, "golden_scanning", Some("golden_scan_hint"), Button::None, false);
        send(app, vec![cmd::ear_scan(true, uid)]);
        app.hearing.timeout = Some(Instant::now() + Duration::from_secs(15));
    } else {
        send(app, vec![cmd::hearing_test(true)]);
        begin(app, 0);
    }
}

fn begin(app: &mut App, i: usize) {
    app.hearing.step = Step::Test;
    app.hearing.index = i;
    app.hearing.toned = None;
    app.hearing.stop = HEARING_START as f32;
    show_text(app, if i < 6 { "golden_left" } else { "golden_right" }, Some(""), Button::Next, true);
    app.hearing.message = t(&app.tr, "golden_step").replace("%1$d", &(i % 6 + 1).to_string());
    play_tone(app);
}

fn next(app: &mut App) {
    if app.hearing.step != Step::Test { return; }
    let stop = app.hearing.stop as usize;
    let i = app.hearing.index;
    app.hearing.values[i] = hearing_snap(HEARING_STOPS[stop.min(24)]);
    if i < 11 { return begin(app, i + 1); }
    app.hearing.step = Step::Finish;
    app.hearing.tone_at = None;
    let (uid, values) = (app.hearing.uid, app.hearing.values);
    show_text(app, "golden_creating", Some(""), Button::None, false);
    send(app, vec![cmd::tone_stop(), cmd::hearing_test(false), cmd::golden_filter(uid, &values)]);
    app.hearing.timeout = Some(Instant::now() + Duration::from_secs(3));
}

fn save(app: &mut App, desc: u8) {
    let h = &mut app.hearing;
    h.step = Step::Done;
    h.timeout = None;
    let r = Record { uid: h.uid, name: now_name(), values: h.values, scan: h.scan_data.clone(), desc };
    put(r.clone());
    apply_record(app, r);
    show_text(app, "golden_saved", Some("golden_saved_msg"), Button::None, false);
    app.hearing.done = true;
}

/// At most one tone every 300 ms while dragging; the last position always plays.
fn tone_soon(app: &mut App) {
    if app.hearing.step != Step::Test { return; }
    let wait = app.hearing.last_tone.map_or(0, |t| 300u64.saturating_sub(t.elapsed().as_millis() as u64));
    if wait == 0 { play_tone(app); } else { app.hearing.tone_at = Some(Instant::now() + Duration::from_millis(wait)); }
}

fn play_tone(app: &mut App) {
    if app.hearing.step != Step::Test { return; }
    let stop = (app.hearing.stop as usize).min(24);
    let h = &mut app.hearing;
    h.last_tone = Some(Instant::now());
    h.toned = Some(stop);
    h.tone_at = None;
    let warn = stop >= HEARING_LOUD && !h.warned;
    h.warned |= warn;
    let i = h.index;
    if warn { h.message = t(&app.tr, "golden_loud").into(); }
    send(app, vec![cmd::tone_stop(), cmd::tone(i as u8 / 6 + 1, i as u8 % 6 + 1, HEARING_STOPS[stop])]);
}

fn stop(app: &mut App, reason: &str) {
    abort(app);
    show_text(app, "golden_stopped", Some(reason), Button::Again, false);
}

/// Stops whatever runs on the buds.
fn abort(app: &mut App) {
    let (step, uid) = (app.hearing.step, app.hearing.uid);
    app.hearing.tone_at = None;
    app.hearing.timeout = None;
    match step {
        Step::Scan => send(app, vec![cmd::ear_scan(false, uid)]),
        Step::Test => send(app, vec![cmd::tone_stop(), cmd::hearing_test(false)]),
        _ => {}
    }
    if step != Step::Done { app.hearing.step = Step::Idle; }
}

/// Applies a record as HeyMelody does (description id, record, ear scan, switch on) and shows its radar.
fn apply_record(app: &mut App, r: Record) {
    send(app, cmd::golden_apply(r.uid, &r.name, &r.values, &r.scan, r.desc));
    app.hearing.active = r.uid;
    request_curves(app);
}

/// Asks the buds for the active record's filters (queries only), unless they are known.
fn request_curves(app: &mut App) {
    let uid = app.hearing.active;
    let Some(r) = records().into_iter().find(|r| r.uid == uid) else { return };
    if app.hearing.hearing.contains_key(&uid) { return; }
    let mut frames = vec![cmd::golden_filter(r.uid, &r.values)];
    if !r.scan.is_empty() { frames.push(cmd::golden_scan_filter(r.uid, &r.scan)); }
    send(app, frames);
}

fn event(app: &mut App, e: HearingEv) {
    match e {
        HearingEv::Status(kind, status) => {
            if !matches!(app.hearing.step, Step::Scan | Step::Test) { return; }
            match status {
                1 | 3 => stop(app, "golden_stopped_music"),
                5 => stop(app, "golden_stopped_wear"),
                7 | 10 | 255 if kind == 4 => stop(app, "golden_scan_failed"),
                _ => {}
            }
        }
        HearingEv::Refused(_) => {
            if matches!(app.hearing.step, Step::Scan | Step::Test) { stop(app, "golden_refused"); }
        }
        // The scan is done: stop it and start the hearing test.
        HearingEv::EarScan(uid, data) => {
            if app.hearing.step != Step::Scan || app.hearing.uid != uid { return; }
            app.hearing.timeout = None;
            app.hearing.scan_data = data;
            send(app, vec![cmd::ear_scan(false, uid), cmd::hearing_test(true)]);
            begin(app, 0);
        }
        HearingEv::Filter(uid, enhance, curves) => {
            if let Some((_, l, r)) = curves { app.hearing.hearing.insert(uid, (l, r)); }
            if app.hearing.step == Step::Finish && app.hearing.uid == uid { save(app, desc_id(enhance)); }
        }
        HearingEv::ScanCurves(uid, c) => { app.hearing.scan.insert(uid, c); }
        HearingEv::Active(uid, name, values) => {
            app.hearing.active = uid;
            if !records().iter().any(|r| r.uid == uid) { put(Record { uid, name, values, scan: vec![], desc: 0 }); }
            // After the ear-scan read (right behind this one) has filled the record in.
            app.hearing.curves_at = Some(Instant::now() + Duration::from_millis(600));
        }
        HearingEv::ActiveScan(uid, data) => {
            let mut list = records();
            if let Some(r) = list.iter_mut().find(|r| r.uid == uid && r.scan.is_empty()) { r.scan = data; save_records(&list); }
        }
    }
}

/// Every frame: the events from the buds, and the deadlines.
pub fn tick(app: &mut App) {
    let events: Vec<HearingEv> = std::mem::take(&mut *HEARING.lock().unwrap());
    for e in events { event(app, e); }
    let now = Instant::now();
    if app.hearing.timeout.is_some_and(|d| now >= d) {
        app.hearing.timeout = None;
        match app.hearing.step {
            Step::Scan => stop(app, "golden_scan_failed"),
            Step::Finish => save(app, 0),
            _ => {}
        }
    }
    if app.hearing.tone_at.is_some_and(|d| now >= d) { play_tone(app); }
    if app.hearing.curves_at.is_some_and(|d| now >= d) { app.hearing.curves_at = None; request_curves(app); }
}

/// The last radar drawn, `[uid, left, right]`, shown until the buds' filters arrive.
fn radii(h: &State) -> Option<[[f32; 6]; 2]> {
    let active = h.active;
    if let Some((l, r)) = h.hearing.get(&active) {
        let sc = h.scan.get(&active);
        let both = [hearing_radar(l, sc.map(|c| (&c.1[..], c.0))), hearing_radar(r, sc.map(|c| (&c.2[..], c.0)))];
        let saved = load_settings()["hearing_radar"].clone();
        let fresh = json!([active, both[0].to_vec(), both[1].to_vec()]);
        if saved != fresh { save_setting("hearing_radar", fresh); }
        return Some(both);
    }
    let v = load_settings()["hearing_radar"].clone();
    if v[0].as_u64() != Some(active as u64) || active == 0 { return None; }
    let ear = |i: usize| -> Option<[f32; 6]> {
        v[i].as_array()?.iter().map(|x| x.as_f64().map(|x| x as f32)).collect::<Option<Vec<_>>>()?.try_into().ok()
    };
    Some([ear(1)?, ear(2)?])
}

/// HeyMelody's radar: the grid, the rim ("Boost") and the profile, in the plot's own points.
fn radar(ui: &Ui, rect: Rect, values: Option<[f32; 6]>) {
    let (c, rad) = (rect.center(), rect.width().min(rect.height()) / 2.0 - 44.0);
    let point = |i: usize, r: f32| {
        let a = -2.0 * std::f32::consts::PI / 3.0 + i as f32 * std::f32::consts::PI / 3.0;
        Pos2::new(c.x + a.cos() * rad * r / 10.0, c.y + a.sin() * rad * r / 10.0)
    };
    let poly = |v: [f32; 6]| -> Vec<Pos2> { v.iter().enumerate().map(|(i, r)| point(i, *r)).collect() };
    for r in [2.5, 5.0, 7.5] { ui.painter().add(Shape::closed_line(poly([r; 6]), Stroke::new(1.0, white(0.2)))); }
    for i in 0..6 { ui.painter().line_segment([c, point(i, 10.0)], Stroke::new(1.0, white(0.2))); }
    ui.painter().add(Shape::closed_line(poly([10.0; 6]), Stroke::new(1.5, white(0.5))));
    if let Some(v) = values {
        ui.painter().add(Shape::convex_polygon(poly(v), white(0.18), Stroke::new(2.5, white(1.0))));
    }
    for (i, f) in HEARING_AXES.iter().enumerate() {
        let text = if *f >= 1000 { format!("{} kHz", *f as f32 / 1000.0) } else { format!("{f} Hz") };
        ui.painter().text(point(i, 12.2), Align2::CENTER_CENTER, text, FontId::proportional(11.5), white(0.75));
    }
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    tick(app);
    ui.ctx().request_repaint_after(Duration::from_millis(200));
    let s = app.snap.clone();
    let tr = app.tr.clone();
    let w = body.width().min(MAX_W);
    let x0 = body.left();
    let mut y = body.top();
    let on = s.status == Status::On;
    let sw = s.features.iter().any(|f| f.0 == FEATURE_HEARING && f.1 == 1);
    let has_test = s.caps.supports(CMD_GOLDEN_DETECT);

    let n = 1 + has_test as usize;
    let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 82.0 * n as f32 - 10.0));
    let r0 = Rect::from_min_size(card.min, Vec2::new(w, 72.0));
    glass::flat_at(ui, r0, 28.0, 1.0);
    let (_, toggled) = glass::row_at(ui, "hearing", r0, icons::HEARING, t(&tr, "row_golden_title"), t(&tr, "row_golden_sub"), Trailing::Switch(sw), on);
    if toggled { app.send(Cmd::Features(vec![(FEATURE_HEARING, !sw)])); }
    if has_test {
        let r1 = Rect::from_min_size(Pos2::new(x0, y + 82.0), Vec2::new(w, 72.0));
        glass::flat_at(ui, r1, 28.0, 1.0);
        let (clicked, _) = glass::row_at(ui, "htest", r1, icons::EARBUD, t(&tr, "golden_test_row"), t(&tr, "golden_test_sub"), Trailing::Chevron, on);
        if clicked {
            app.hearing.open = true;
            app.hearing.step = Step::Idle;
            show_text(app, "golden_test_title", Some("golden_prepare"), Button::Start, false);
        }
    }
    y += card.height() + 18.0;

    // Profiles
    let list = records();
    glass::text(ui, Pos2::new(x0 + 6.0, y + 14.0), Align2::LEFT_CENTER, t(&tr, "golden_profiles"), 14.0, 0.72);
    y += 34.0;
    if list.is_empty() {
        let r = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 64.0));
        glass::flat_at(ui, r, 28.0, 1.0);
        glass::text(ui, r.center(), Align2::CENTER_CENTER, t(&tr, "golden_none"), 14.0, 0.6);
        y += 64.0;
    } else {
        let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, (ROW + 10.0) * list.len() as f32 - 10.0));
        let active = app.hearing.active;
        for (i, r) in list.iter().enumerate() {
            let row = Rect::from_min_size(Pos2::new(x0, y + (ROW + 10.0) * i as f32), Vec2::new(w, ROW));
            glass::flat_at(ui, row, 28.0, 1.0);
            if glass::list_item_at(ui, ("rec", r.uid), row, &r.name, r.uid == active, !on).clicked() && on { apply_record(app, r.clone()); }
            let del = Rect::from_center_size(Pos2::new(row.right() - 28.0, row.center().y), Vec2::splat(36.0));
            let resp = ui.interact(del, ui.scope_id().with(("recdel", r.uid)), egui::Sense::click());
            glass::icon_at(ui, icons::DELETE, Rect::from_center_size(del.center(), Vec2::splat(20.0)), white(if resp.hovered() { 1.0 } else { 0.65 }));
            if resp.clicked() { app.hearing.delete = Some(i); }
        }
        y += card.height();
    }
    y += 18.0;

    // The radar of the active profile
    let both = radii(&app.hearing);
    if let Some(both) = both {
        for (i, label) in ["golden_left", "golden_right"].iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(x0 + i as f32 * (w / 2.0 + 4.0), y), Vec2::new(w / 2.0 - 4.0, 44.0));
            let lit = app.hearing.ear == i;
            if glass::button_at(ui, ("ear", i), r, 22.0, lit).clicked() { app.hearing.ear = i; }
            ui.painter().text(r.center(), Align2::CENTER_CENTER, t(&tr, label), FontId::proportional(14.0), if lit { glass::on_accent() } else { glass::white(1.0) });
        }
        y += 44.0 + 12.0;
        let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 380.0));
        glass::flat_at(ui, card, 28.0, 1.0);
        radar(ui, card, Some(both[app.hearing.ear.min(1)]));
        y += card.height() + 12.0;
    }

    // The delete question
    if let Some(i) = app.hearing.delete {
        let Some(r) = list.get(i) else { app.hearing.delete = None; return y };
        let msg = t(&tr, "eq_delete_confirm").replace("%1$s", &r.name);
        let mut decision = None;
        let open = glass::dialog(ui.ctx(), "hear-del", t(&tr, "eq_delete"), Vec2::new(420.0, 200.0), |ui| {
            ui.add(egui::Label::new(egui::RichText::new(&msg).size(15.0).color(white(0.9))).wrap());
            decision = glass::dialog_buttons(ui, t(&tr, "dialog_cancel"), t(&tr, "eq_delete"));
        });
        match decision {
            Some(true) => {
                let mut all = records();
                if i < all.len() { all.remove(i); }
                save_records(&all);
                app.hearing.delete = None;
            }
            Some(false) => app.hearing.delete = None,
            None => if !open { app.hearing.delete = None },
        }
    }

    test_dialog(app, ui);
    y
}

/// The test: title, message, the volume slider while a tone plays, the main button; closing aborts it.
fn test_dialog(app: &mut App, ui: &mut Ui) {
    if !app.hearing.open { return; }
    let tr = app.tr.clone();
    let h = &mut app.hearing;
    let (title, message, slider, done) = (h.title.clone(), h.message.clone(), h.slider, h.done);
    let label = match h.button {
        Button::Start => t(&tr, "golden_start").to_string(),
        Button::Again => t(&tr, "golden_again").to_string(),
        Button::Next => t(&tr, if h.index < 11 { "golden_next" } else { "golden_finish" }).to_string(),
        Button::None => "…".to_string(),
    };
    let label = if done { t(&tr, "gesture_done").to_string() } else { label };
    let mut stop_v = h.stop;
    let mut moved = false;
    let mut decision = None;
    let open = glass::dialog(ui.ctx(), "hear-test", &title, Vec2::new(460.0, 300.0), |ui| {
        let r = ui.max_rect();
        ui.add(egui::Label::new(egui::RichText::new(&message).size(14.0).color(white(0.85))).wrap());
        if slider {
            let sr = Rect::from_min_size(Pos2::new(r.left(), r.top() + 90.0), Vec2::new(r.width(), 44.0));
            moved = glass::slider_at(ui, "hstop", sr, &mut stop_v, 0.0, 24.0, true).changed;
        }
        decision = glass::dialog_buttons(ui, t(&tr, "dialog_close"), &label);
    });
    app.hearing.stop = stop_v;
    if moved && app.hearing.toned != Some(stop_v as usize) { tone_soon(app); }
    match decision {
        Some(true) => {
            if app.hearing.done { app.hearing.open = false; }
            else { match app.hearing.button { Button::Start | Button::Again => start(app), Button::Next => next(app), Button::None => {} } }
        }
        Some(false) => close(app),
        None => if !open { close(app) },
    }
}

/// Closed while the enhance type was being read: the result is still kept.
fn close(app: &mut App) {
    if app.hearing.step == Step::Finish { save(app, 0); } else { abort(app); }
    app.hearing.open = false;
}
