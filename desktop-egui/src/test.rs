//! Test my earbuds (`ModelTestActivity`, DESIGN §7.15): Start, the automatic check, a mark per feature, the
//! diagnosis, the JSON preview, and the machine-readable line the report carries. The rule (which features,
//! which checks, how the two replies differ) is the one of `desktop/src/modeltest.rs`; a test reads that file
//! and fails when the two lists drift.

use crate::glass::{self, white};
use crate::phone;
use crate::report;
use crate::session::{Cmd, Status, LOG};
use crate::settings::sheets::{self, Title, WIDTH};
use crate::settings::ui::{fit, section};
use crate::types::{self, cmd_of, EVT_PUSH};
use crate::{t, App};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Ui, Vec2};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::time::{Duration, Instant};

const MAX_W: f32 = 379.4;
/// The buds need a moment between the drop and the new connect.
const RECONNECT_GAP: Duration = Duration::from_millis(1500);
/// No connection by then: the test says so and Start works again.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(25);
/// After the connect, the handshake's reads arrive within this time.
const HANDSHAKE: Duration = Duration::from_secs(6);
/// The start of the machine-readable line. `scripts/import-model-test.py` finds the JSON after it.
const JSON_PREFIX: &str = "MODEL TEST JSON ";

const WORKS: u8 = 1;
const BROKEN: u8 = 2;
const SKIP: u8 = 3;

/// (key in `models.json` `confirmed`, label, name in the report, the buds have it): the phone's `ModelTest.FEATURES`.
type Feature = (&'static str, &'static str, &'static str, fn(&App) -> bool);

const FEATURES: &[Feature] = &[
    ("battery", "problem_cat_battery", "Battery", |_| true),
    ("anc", "anc_section", "Noise control", |a| {
        let m = &a.snap.modes;
        m.supports(types::OFF) || !m.levels().is_empty() || m.supports(types::ADAPTIVE) || m.supports(types::TRANSPARENCY)
    }),
    ("wind", "row_wind_noise_title", "Wind noise reduction", |a| {
        listed(a, types::FEATURE_WIND_NOISE) || a.snap.manual && a.snap.model.is_some_and(|m| m["windMode"] == 1)
    }),
    ("game", "row_game_title", "Low latency", |a| types::has_feature(&a.snap.features, a.snap.caps.game_mode_id())),
    ("hires", "row_hires_title", "High-quality audio", |a| listed(a, types::FEATURE_HIRES) || crate::home_logic::codec_picker(&a.snap)),
    ("spatial", "row_spatial_title", "3D audio", |a| listed(a, types::FEATURE_SPATIAL) || crate::home_logic::spatial_by_type(&a.snap)),
    ("golden", "row_golden_title", "Hearing profile", |a| a.has.hearing),
    ("eq", "row_eq_title", "Equalizer", |a| a.has.eq),
    ("dual", "dual_title", "Dual connection", |a| a.has.dual),
    ("gestures", "gesture_title", "Gestures", |a| a.has.controls),
    ("wear", "wear_firmware_title", "Wear detection", |a| listed(a, 0x04)),
    ("find", "row_find_title", "Find my earbuds", |a| a.snap.caps.supports(types::CMD_FIND_BUDS)),
];

/// The automatic check: (key, label, name in the report).
const CHECKS: [(&str, &str, &str); 3] = [
    ("battery", "test_check_battery", "battery level"),
    ("features", "test_check_features", "feature list"),
    ("states", "test_check_states", "feature states"),
];

/// The row key of a feature key, where the two differ. A feature without a row (battery) is absent.
const ROW_KEY: &[(&str, &str)] = &[
    ("wind", "wind"), ("game", "game"), ("hires", "hires"), ("spatial", "spatial"), ("golden", "golden"),
    ("eq", "eq"), ("dual", "dual"), ("anc", "anc"), ("gestures", "controls"), ("wear", "wear"), ("find", "find"),
];

#[derive(Default, Clone, Copy, PartialEq)]
enum Phase { #[default] Idle, Dropping, Connecting, Reading }

#[derive(Default)]
pub struct State {
    pub started: bool,
    phase: Phase,
    /// The Connect went out; the link should come up.
    connect_sent: bool,
    deadline: Option<Instant>,
    checks: [bool; 3],
    marks: HashMap<&'static str, u8>,
    notes: HashMap<&'static str, String>,
    marking: Option<&'static str>,
    note_for: Option<&'static str>,
    note_text: String,
    pub preview: bool,
    sending: bool,
    status: String,
    job: Option<report::Job>,
}

fn listed(a: &App, id: u8) -> bool { a.snap.features.iter().any(|f| f.0 == id) }

fn shown(a: &App) -> Vec<&'static Feature> { FEATURES.iter().filter(|f| (f.3)(a)).collect() }

fn mark_label(tr: &[String], m: u8) -> &str {
    match m { WORKS => t(tr, "test_works"), BROKEN => t(tr, "test_broken"), SKIP => t(tr, "test_skipped"), _ => t(tr, "test_not_marked") }
}

fn frames() -> bool { LOG.lock().unwrap().iter().any(|l| l.dir == "RX") }

/// Every feature shown has a mark, and every "does not work" has a note.
fn complete(app: &App) -> bool {
    shown(app).iter().all(|f| match app.test.marks.get(f.0) {
        None | Some(0) => false,
        Some(&BROKEN) => app.test.notes.get(f.0).is_some_and(|n| !n.trim().is_empty()),
        _ => true,
    })
}

fn ready(app: &App) -> bool { complete(app) && frames() }

/// Start: clear the log, drop the link and connect again.
fn start(app: &mut App) {
    if app.test.phase != Phase::Idle { return; }
    LOG.lock().unwrap().clear();
    app.test.marks.clear();
    app.test.notes.clear();
    app.test.status = t(&app.tr, "test_reconnecting").to_string();
    app.test.deadline = Some(Instant::now() + CONNECT_TIMEOUT);
    if app.snap.status == Status::Off {
        app.test.phase = Phase::Connecting;
        app.test.connect_sent = false;
        app.test.deadline = Some(Instant::now() + RECONNECT_GAP);
    } else {
        app.test.phase = Phase::Dropping;
        app.send(Cmd::Disconnect);
    }
}

fn fail(app: &mut App) {
    app.test.phase = Phase::Idle;
    app.test.connect_sent = false;
    app.test.deadline = None;
    app.test.started = false;
    app.test.status = t(&app.tr, "test_failed").to_string();
}

/// The check from the fresh connection's state, then the feature rows.
fn finish(app: &mut App) {
    let s = &app.snap;
    app.test.checks = [s.battery.iter().any(Option::is_some), s.caps.read(), !s.features.is_empty()];
    app.test.phase = Phase::Idle;
    app.test.connect_sent = false;
    app.test.deadline = None;
    app.test.status.clear();
}

/// Each frame: the reconnect steps and their timeouts.
fn tick(app: &mut App) {
    let now = Instant::now();
    let due = app.test.deadline.is_some_and(|d| now >= d);
    match app.test.phase {
        Phase::Dropping if app.snap.status == Status::Off => {
            app.test.phase = Phase::Connecting;
            app.test.deadline = Some(now + RECONNECT_GAP);
        }
        Phase::Dropping if due => fail(app),
        Phase::Connecting if !app.test.connect_sent && due => {
            app.send(Cmd::Connect);
            app.test.connect_sent = true;
            app.test.deadline = Some(now + CONNECT_TIMEOUT);
        }
        Phase::Connecting if app.test.connect_sent && app.snap.status == Status::On => {
            app.test.phase = Phase::Reading;
            app.test.connect_sent = false;
            app.test.deadline = Some(now + HANDSHAKE);
        }
        Phase::Connecting if app.test.connect_sent && due => fail(app),
        Phase::Reading if due => finish(app),
        _ => {}
    }
}

struct Diff { missing: Vec<u8>, extra: Vec<u8>, hidden: Vec<&'static str> }

impl Diff {
    fn empty(&self) -> bool { self.missing.is_empty() && self.extra.is_empty() && self.hidden.is_empty() }
}

/// The two game-mode ids are alternatives: when the app asked both and the reply holds one, the other is no
/// missing answer. Both ids must be in the ask list for this to apply.
fn other_game_id(answered: &HashSet<u8>, asked: &[u8], id: u8) -> bool {
    let alt = match id { 0x06 => 0x28, 0x28 => 0x06, _ => return false };
    answered.contains(&alt) && asked.contains(&alt)
}

fn diff(asked: &[u8], reply: Option<&[(u8, u8)]>, rows: &BTreeSet<&'static str>,
        marks: &HashMap<&'static str, u8>) -> Diff {
    let missing = reply.map(|r| {
        let answered: HashSet<u8> = r.iter().map(|f| f.0).collect();
        let mut v: Vec<u8> = asked.iter().copied()
            .filter(|id| !answered.contains(id) && !other_game_id(&answered, asked, *id))
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }).unwrap_or_default();
    let extra = reply.map(|r| {
        let mut v: Vec<u8> = r.iter().map(|f| f.0).filter(|id| !asked.contains(id)).collect();
        v.sort_unstable();
        v.dedup();
        v
    }).unwrap_or_default();
    let hidden = FEATURES.iter()
        .filter(|f| marks.get(f.0) == Some(&WORKS))
        .filter(|f| ROW_KEY.iter().find(|(k, _)| *k == f.0).is_some_and(|(_, row)| !rows.contains(row)))
        .map(|f| f.0)
        .collect();
    Diff { missing, extra, hidden }
}

/// The rows this app shows for the features the test lists, and the Earbud settings row it always shows.
fn rows(a: &App) -> BTreeSet<&'static str> {
    let mut out = BTreeSet::from(["earbuds"]);
    out.extend(shown(a).iter().filter_map(|f| ROW_KEY.iter().find(|(k, _)| *k == f.0).map(|(_, row)| *row)));
    out
}

/// [diff] from the state of the test in progress: the ids this app asks, the last status reply, the rows, the marks.
fn diff_state(a: &App) -> Diff {
    let mut asked = types::status_query(a.snap.model);
    if !asked.is_empty() { asked.remove(0); }
    let reply = (!a.snap.features.is_empty()).then_some(a.snap.features.as_slice());
    diff(&asked, reply, &rows(a), &a.test.marks)
}

fn mark_name(m: u8) -> Option<&'static str> {
    Some(match m { WORKS => "works", BROKEN => "broken", SKIP => "skip", _ => return None })
}

/// The command ids of the RX frames the app did not decode.
fn unknown_rx() -> BTreeSet<u16> {
    LOG.lock().unwrap().iter()
        .filter(|l| l.dir == "RX" && l.human.is_none())
        .map(|l| cmd_of(&l.bytes))
        .filter(|c| *c != EVT_PUSH)
        .collect()
}

/// The one machine-readable line of this test: the model, the firmware, the two replies, the rows shown and
/// expected, the marks, and the commands the app did not decode. Same keys and formats as the phone and
/// `desktop/src/modeltest.rs`.
fn json_line(app: &App) -> String {
    let keys: Vec<&'static str> = shown(app).iter().map(|f| f.0).collect();
    let name = app.snap.model.and_then(|m| m["name"].as_str()).unwrap_or("unknown model");
    let id = crate::load_settings()["product_id"].as_str().unwrap_or_default().to_string();
    let mut caps = app.snap.caps.0.clone().unwrap_or_default();
    caps.sort_unstable();
    caps.dedup();
    let caps = caps.iter().map(|c| format!("{c:04X}")).collect::<Vec<_>>().join(" ");
    let status: serde_json::Map<String, serde_json::Value> =
        app.snap.features.iter().map(|(id, v)| (format!("{id:02X}"), (*v).into())).collect();
    let marks: serde_json::Map<String, serde_json::Value> = keys.iter()
        .filter_map(|k| app.test.marks.get(k).and_then(|m| mark_name(*m)).map(|n| (k.to_string(), n.into())))
        .collect();
    let json = serde_json::json!({
        "id": id,
        "name": name,
        "fw": app.snap.firmware.as_deref().unwrap_or(""),
        "caps": caps,
        "status": status,
        "shown": keys,
        "expected": rows(app).iter().collect::<Vec<_>>(),
        "marks": marks,
        "unknownRx": unknown_rx().iter().map(|c| format!("0x{c:04X}")).collect::<Vec<_>>(),
    });
    format!("{JSON_PREFIX}{json}")
}

/// The report's description: the check, the diagnosis, then one line per feature.
fn description(app: &App) -> String {
    let check: Vec<String> = CHECKS.iter().zip(app.test.checks)
        .map(|((_, _, name), ok)| format!("{name} {}", if ok { "answered" } else { "no answer" })).collect();
    let mut lines = vec![
        format!("Model test ({})", if cfg!(windows) { "Windows" } else { "Linux" }),
        format!("Automatic check: {}", check.join(", ")),
    ];
    let d = diff_state(app);
    let ids = |v: &[u8]| v.iter().map(|id| format!("0x{id:02X}")).collect::<Vec<_>>().join(" ");
    let mut parts = vec![];
    if !d.missing.is_empty() { parts.push(format!("asked, no answer: {}", ids(&d.missing))); }
    if !d.extra.is_empty() { parts.push(format!("answered, not asked: {}", ids(&d.extra))); }
    if !d.hidden.is_empty() {
        let names: Vec<&str> = FEATURES.iter().filter(|f| d.hidden.contains(&f.0)).map(|f| f.2).collect();
        parts.push(format!("marked works, no row: {}", names.join(", ")));
    }
    if !parts.is_empty() { lines.push(format!("Diagnosis: {}", parts.join("; "))); }
    for f in shown(app) {
        lines.push(match app.test.marks.get(f.0) {
            Some(&WORKS) => format!("{}: works", f.2),
            Some(&BROKEN) => format!("{}: does not work: {}", f.2, app.test.notes.get(f.0).map_or("", |n| n.as_str())),
            _ => format!("{}: skipped", f.2),
        });
    }
    lines.join("\n")
}

fn draft(app: &App) -> report::Draft {
    report::Draft {
        model: app.snap.model.and_then(|m| m["name"].as_str()).unwrap_or("unknown model").to_string(),
        categories: vec!["Test"],
        description: description(app),
        log: Some(report::log_text(app)),
        crash: false,
    }
}

fn send(app: &mut App, ctx: &egui::Context) {
    if !ready(app) || app.test.sending { return; }
    // The machine-readable line goes in first, so the log this report sends carries it.
    let line = json_line(app);
    crate::session::log("NOTE", Vec::new(), Some(line));
    app.test.job = Some(report::send_async(draft(app), ctx));
    app.test.sending = true;
    app.test.status.clear();
}

/// The diagnosis lines, as the phone writes them.
fn diagnosis(app: &App, tr: &[String]) -> Vec<String> {
    let d = diff_state(app);
    let ids = |v: &[u8]| v.iter().map(|id| format!("0x{id:02X}")).collect::<Vec<_>>().join(" ");
    let mut lines = vec![];
    if !d.missing.is_empty() { lines.push(t(tr, "test_diff_missing").replace("%1$s", &ids(&d.missing))); }
    if !d.extra.is_empty() { lines.push(t(tr, "test_diff_extra").replace("%1$s", &ids(&d.extra))); }
    if !d.hidden.is_empty() {
        let names: Vec<&str> = FEATURES.iter().filter(|f| d.hidden.contains(&f.0)).map(|f| t(tr, f.1)).collect();
        lines.push(t(tr, "test_diff_hidden").replace("%1$s", &names.join(", ")));
    }
    lines
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    tick(app);
    // The answer of a Send, on a later frame after the network thread posts it.
    if let Some((ok, refused)) = app.test.job.as_ref().and_then(|j| j.poll()) {
        app.test.job = None;
        app.test.sending = false;
        if ok {
            app.test.marks.clear();
            app.test.notes.clear();
            app.test.started = false;
            app.test.status = t(&app.tr, "problem_sent").to_string();
        } else {
            app.test.status = refused.unwrap_or_else(|| t(&app.tr, "problem_failed").to_string());
        }
    }
    if app.test.phase != Phase::Idle || app.test.sending { ui.ctx().request_repaint_after(Duration::from_millis(250)); }
    let tr = app.tr.clone();
    let w = phone::content(body).min(MAX_W);
    let x0 = body.left();
    let mut y = body.top() + 0.2;

    phone::paragraph(ui, Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 44.0)), t(&tr, "test_intro"), 13.0, white(0.74), false);
    y += 42.3 + 12.0;
    let start_rect = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 52.0));
    let label = if app.test.started || app.test.phase != Phase::Idle { t(&tr, "test_restart") } else { t(&tr, "test_start") };
    if phone::pill(ui, "test-start", start_rect, label, false, crate::palette::active(false, false).accent) {
        start(app);
        app.test.started = true;
        app.test.preview = false;
    }
    y += 52.0 + 16.0;
    if !app.test.status.is_empty() {
        phone::paragraph(ui, Rect::from_min_size(Pos2::new(x0 + 4.0, y), Vec2::new(w - 8.0, 34.0)), &app.test.status, 13.0, white(0.74), false);
        y += 34.0;
    }

    if app.test.started {
        section(ui, x0, y, t(&tr, "test_section_check"), white(0.70));
        y += phone::SECTION;
        for (i, (_, key, _)) in CHECKS.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 56.0));
            glass::flat_at(ui, r, 16.0, 1.0);
            glass::text(ui, Pos2::new(r.left() + 16.0, r.center().y), Align2::LEFT_CENTER, t(&tr, key), 16.0, 1.0);
            let ok = app.test.phase == Phase::Idle && app.test.checks[i];
            let text = t(&tr, if ok { "test_answered" } else { "test_no_answer" });
            ui.painter().text(Pos2::new(r.right() - 16.0, r.center().y), Align2::RIGHT_CENTER, text, FontId::proportional(13.0),
                if ok { white(0.9) } else { white(0.6) });
            y += 56.0 + 8.0;
        }
        y += 12.0;

        let lines = diagnosis(app, &tr);
        if !lines.is_empty() {
            section(ui, x0, y, t(&tr, "test_section_diff"), white(0.70));
            y += phone::SECTION;
            let heights: Vec<f32> = lines.iter().map(|l| phone::wrap_height(ui, l, 13.0, w - 32.0) + 16.0).collect();
            let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, heights.iter().sum::<f32>()));
            glass::flat_at(ui, card, 16.0, 1.0);
            let mut cy = y;
            for (line, h) in lines.iter().zip(&heights) {
                phone::paragraph(ui, Rect::from_min_size(Pos2::new(x0 + 16.0, cy + 8.0), Vec2::new(w - 32.0, *h)), line, 13.0, white(0.74), false);
                cy += h;
            }
            y = card.bottom() + 12.0;
        }

        section(ui, x0, y, t(&tr, "test_section_features"), white(0.70));
        y += phone::SECTION;
        let features = shown(app);
        for f in &features {
            let rect = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 59.8));
            glass::flat_at(ui, rect, 16.0, 1.0);
            glass::text(ui, Pos2::new(rect.left() + 16.0, rect.top() + 19.6), Align2::LEFT_CENTER,
                &fit(ui, t(&tr, f.1), 16.0, w - 32.0 - 110.0), 16.0, 1.0);
            let m = *app.test.marks.get(f.0).unwrap_or(&0);
            let sub = match m {
                WORKS => t(&tr, "test_works").to_string(),
                BROKEN => format!("{}: {}", t(&tr, "test_broken"), app.test.notes.get(f.0).map_or("", |n| n.as_str())),
                SKIP => t(&tr, "test_skipped").to_string(),
                _ => t(&tr, "test_not_marked").to_string(),
            };
            glass::text(ui, Pos2::new(rect.left() + 16.0, rect.top() + 41.0), Align2::LEFT_CENTER,
                &fit(ui, &sub, 13.0, w - 32.0 - 40.0), 13.0, if m == WORKS { 0.9 } else { 0.74 });
            let hit = ui.interact(rect, ui.scope_id().with(("mark", f.0)), Sense::click());
            if hit.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
            if hit.clicked() { app.test.marking = Some(f.0); }
            y += 59.8 + 8.0;
        }
        y += 4.0;

        let all = complete(app);
        let bw = (w - 12.0) / 2.0;
        let see = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(bw, 52.0));
        let send_rect = Rect::from_min_size(Pos2::new(x0 + bw + 12.0, y), Vec2::new(bw, 52.0));
        let accent = crate::palette::active(false, false).accent;
        if phone::pill(ui, "test-preview", see, t(&tr, "problem_preview"), false, accent) { app.test.preview = true; }
        let label = if app.test.sending { t(&tr, "problem_sending") } else { t(&tr, "problem_send") };
        if phone::pill(ui, "test-send", send_rect, label, ready(app), accent) { send(app, ui.ctx()); }
        y += 52.0 + 12.0;
        let note = if ready(app) { "" } else if all { t(&tr, "problem_log_empty") } else { t(&tr, "test_mark_all") };
        glass::text(ui, Pos2::new(x0 + 4.0, y + 6.0), Align2::LEFT_CENTER, note, 12.0, 0.6);
        y += 24.0;
    }

    mark_sheet(app, ui);
    note_sheet(app, ui);
    if app.test.preview { preview(app, ui); }
    y + 24.0
}

/// Works / Does not work / Skip. Does not work asks what happened.
fn mark_sheet(app: &mut App, ui: &mut Ui) {
    let Some(id) = app.test.marking else { return };
    let tr = app.tr.clone();
    let title = FEATURES.iter().find(|f| f.0 == id).map_or("", |f| t(&tr, f.1));
    let options = [t(&tr, "test_works"), t(&tr, "test_broken"), t(&tr, "test_skip")];
    let cur = *app.test.marks.get(id).unwrap_or(&0);
    let mut decision = None;
    let open = sheets::sheet(ui.ctx(), "mark-sheet", title, Title::Centre, Vec2::new(WIDTH, 330.0), |ui, inner| {
        for (i, label) in options.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(inner.left() + 4.0, inner.top() + 8.0 + 56.0 * i as f32), Vec2::new(inner.width() - 8.0, 56.0));
            if glass::list_item_at(ui, ("mark", i), r, label, cur == i as u8 + 1, false).clicked() { decision = Some(i as u8 + 1); }
        }
        let note = Rect::from_min_max(Pos2::new(inner.left() + 14.0, inner.top() + 8.0 + 56.0 * 3.0), Pos2::new(inner.right() - 14.0, inner.bottom() - 66.0));
        phone::paragraph(ui, note, t(&tr, "test_broken_hint"), 12.0, white(0.6), false);
    });
    match decision {
        Some(BROKEN) => {
            app.test.note_for = Some(id);
            app.test.note_text = app.test.notes.get(id).cloned().unwrap_or_default();
            app.test.marking = None;
        }
        Some(m) => { app.test.marks.insert(id, m); app.test.marking = None; }
        None if !open => app.test.marking = None,
        None => {}
    }
}

/// The note a "does not work" mark needs.
fn note_sheet(app: &mut App, ui: &mut Ui) {
    let Some(id) = app.test.note_for else { return };
    let tr = app.tr.clone();
    let title = FEATURES.iter().find(|f| f.0 == id).map_or("", |f| t(&tr, f.1));
    let mut decision = None;
    let open = sheets::sheet(ui.ctx(), "note-sheet", title, Title::Centre, Vec2::new(WIDTH, 300.0), |ui, inner| {
        let hint = Rect::from_min_size(inner.min + Vec2::new(14.0, 0.0), Vec2::new(inner.width() - 28.0, 34.0));
        phone::paragraph(ui, hint, t(&tr, "test_broken_hint"), 13.0, white(0.74), false);
        let field = Rect::from_min_size(Pos2::new(inner.left() + 14.0, hint.bottom() + 8.0), Vec2::new(inner.width() - 28.0, 50.7));
        phone::field_at(ui, field, &mut app.test.note_text, 300);
        decision = sheets::actions(ui, inner, t(&tr, "dialog_cancel"), t(&tr, "test_broken"));
    });
    match decision {
        Some(true) if !app.test.note_text.trim().is_empty() => {
            app.test.marks.insert(id, BROKEN);
            app.test.notes.insert(id, app.test.note_text.trim().to_string());
            app.test.note_for = None;
        }
        Some(true) => {}
        Some(false) => app.test.note_for = None,
        None if !open => app.test.note_for = None,
        None => {}
    }
}

fn preview(app: &mut App, ui: &mut Ui) {
    let tr = app.tr.clone();
    let body = report::preview(&draft(app));
    let ok = ready(app);
    let mut decision = None;
    let open = sheets::sheet(ui.ctx(), "test-preview", t(&tr, "problem_preview"), Title::Centre, Vec2::new(WIDTH, 420.0), |ui, inner| {
        let log = Rect::from_min_max(inner.min + Vec2::new(14.0, 0.0), Pos2::new(inner.right() - 14.0, inner.bottom() - 66.0));
        glass::flat_at(ui, log, 12.0, 1.0);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(log.shrink(10.0)));
        child.set_clip_rect(log.shrink(10.0));
        egui::ScrollArea::vertical().show(&mut child, |ui| {
            ui.add(egui::Label::new(egui::RichText::new(&body).monospace().size(12.0).color(white(0.8))));
        });
        decision = sheets::actions(ui, inner, t(&tr, "dialog_close"), if ok { t(&tr, "problem_send") } else { "" });
    });
    if decision == Some(true) && ok {
        app.test.preview = false;
        send(app, ui.ctx());
    } else if decision.is_some() || !open {
        app.test.preview = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The phone lists the same keys in the same order, so both apps send the same report.
    #[test]
    fn same_keys_as_phone() {
        let kt = include_str!("../../app/src/main/kotlin/com/spizganed/quickbuds/ui/ModelTest.kt");
        let phone: Vec<&str> = kt.split("Feature(\"").skip(1).map(|s| &s[..s.find('"').unwrap()]).collect();
        let here: Vec<&str> = FEATURES.iter().map(|f| f.0).collect();
        assert_eq!(phone, here);
    }

    /// `desktop/src/modeltest.rs` holds the same rule; the two lists must not drift while both apps exist.
    #[test]
    fn same_rule_as_the_slint_build() {
        // The three quoted words on the first line of each entry: key, label, report name.
        fn entries(block: &str) -> Vec<String> {
            block.lines()
                .filter(|l| l.trim_start().starts_with("(\""))
                .flat_map(|l| l.split('"').skip(1).step_by(2).take(3).map(String::from).collect::<Vec<_>>())
                .collect()
        }
        let src = include_str!("../../desktop/src/modeltest.rs");
        let features = &src[src.find("const FEATURES").unwrap()..src.find("const CHECKS").unwrap()];
        let ours: Vec<String> = FEATURES.iter().flat_map(|f| [f.0, f.1, f.2].map(String::from)).collect();
        assert_eq!(entries(features), ours);
        let checks = &src[src.find("const CHECKS").unwrap()..src.find("/// A mark, as the page's").unwrap()];
        let ours: Vec<String> = CHECKS.iter().flat_map(|c| [c.0, c.1, c.2].map(String::from)).collect();
        assert_eq!(entries(checks), ours);
    }

    #[test]
    fn labels_exist() {
        for key in FEATURES.iter().map(|f| f.1).chain(CHECKS.iter().map(|c| c.1)) {
            assert!(crate::strings::KEYS.contains(&key), "{key}");
        }
    }

    /// The diagnosis: one line, the three mismatches in the phone's words.
    #[test]
    fn diagnosis_text() {
        let d = diff(&[0x04, 0x06], Some(&[(0x28, 1)]), &BTreeSet::from(["earbuds", "hires"]),
            &HashMap::from([("eq", WORKS)]));
        assert_eq!(d.missing, [0x04, 0x06]);
        assert_eq!(d.extra, [0x28]);
        assert_eq!(d.hidden, ["eq"]);
        let clean = diff(&[0x05], Some(&[(0x05, 1)]), &BTreeSet::from(["earbuds"]), &HashMap::new());
        assert!(clean.empty());
        // Nothing read: no missing and no extra.
        assert!(diff(&[0x04, 0x05], None, &BTreeSet::from(["earbuds"]), &HashMap::new()).empty());
    }

    #[test]
    fn one_game_id_is_no_missing_answer() {
        let d = diff(&[0x06, 0x28], Some(&[(0x06, 1)]), &BTreeSet::from(["earbuds"]), &HashMap::new());
        assert!(d.missing.is_empty(), "missing {:?}", d.missing);
    }

    /// The machine-readable line holds the nine fields `scripts/import-model-test.py` reads.
    #[test]
    fn json_line_for_the_import() {
        let line = "MODEL TEST JSON {\"caps\":\"\",\"expected\":[\"earbuds\"],\"fw\":\"\",\"id\":\"\",\"marks\":{},\"name\":\"unknown model\",\"shown\":[],\"status\":{},\"unknownRx\":[]}";
        let v: serde_json::Value = serde_json::from_str(&line[JSON_PREFIX.len()..]).unwrap();
        let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["caps", "expected", "fw", "id", "marks", "name", "shown", "status", "unknownRx"]);
    }
}
