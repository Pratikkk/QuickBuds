//! App settings › Test my earbuds, and its launch prompt: the phone's `ModelTest` and `ModelTestActivity`, with the
//! same feature keys and the same report text. Start clears the log and reconnects; the automatic check notes which
//! reads the buds answered; the person marks each feature; Send posts one "Test" report, always with the log.

use crate::types::*;
use crate::session::{Cmd, Status, LOG};
use crate::{load_settings, save_setting, t, with_app, App, MainWindow, ModelTest as G, TestCheck, TestRow};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::time::Duration;

/// (key in `models.json` `confirmed`, label, name in the report, the buds have it): the phone's `ModelTest.FEATURES`.
type Feature = (&'static str, &'static str, &'static str, fn(&App) -> bool);

const FEATURES: &[Feature] = &[
    ("battery", "problem_cat_battery", "Battery", |_| true),
    ("anc", "anc_section", "Noise control", |a| {
        let m = &a.snap.modes;
        m.supports(OFF) || !m.levels().is_empty() || m.supports(ADAPTIVE) || m.supports(TRANSPARENCY)
    }),
    ("wind", "row_wind_noise_title", "Wind noise reduction", |a| {
        listed(a, FEATURE_WIND_NOISE) || a.snap.manual && a.snap.model.is_some_and(|m| m["windMode"] == 1)
    }),
    ("game", "row_game_title", "Low latency", |a| a.has.game),
    ("hires", "row_hires_title", "High-quality audio", |a| listed(a, FEATURE_HIRES) || crate::earbuds::codec_picker(&a.snap)),
    ("spatial", "row_spatial_title", "3D audio", |a| listed(a, FEATURE_SPATIAL) || crate::earbuds::spatial_by_type(&a.snap)),
    ("golden", "row_golden_title", "Hearing profile", |a| a.has.hearing),
    ("eq", "row_eq_title", "Equalizer", |a| a.has.eq),
    ("dual", "dual_title", "Dual connection", |a| a.has.dual),
    ("gestures", "gesture_title", "Gestures", |a| a.has.controls),
    ("wear", "wear_firmware_title", "Wear detection", |a| listed(a, 0x04)),
    ("find", "row_find_title", "Find my earbuds", |a| a.snap.caps.supports(CMD_FIND_BUDS)),
];

/// The automatic check: (key, label, name in the report).
const CHECKS: [(&str, &str, &str); 3] = [
    ("battery", "test_check_battery", "battery level"),
    ("features", "test_check_features", "feature list"),
    ("states", "test_check_states", "feature states"),
];

/// A mark, as the page's chips number them.
const WORKS: i32 = 1;
const BROKEN: i32 = 2;
const SKIP: i32 = 3;

/// The buds need a moment between the drop and the new connect.
const RECONNECT_GAP: Duration = Duration::from_millis(1500);
/// No connection by then: the test says so and Start works again.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(25);
/// After the connect, the handshake's reads (battery, `0x8100`, `0x810D`) arrive within this time.
const HANDSHAKE: Duration = Duration::from_secs(6);
/// After a connect, the prompt waits for the reads that decide which features show.
const ASK_DELAY: Duration = Duration::from_secs(3);

fn listed(a: &App, id: u8) -> bool { a.snap.features.iter().any(|f| f.0 == id) }

#[derive(Clone, Copy, PartialEq)]
enum Phase { Idle, Dropping, Connecting, Reading }

/// The test in progress for one model: kept while the person tries features on other pages.
struct State {
    model: String,
    checks: [bool; 3],
    marks: HashMap<&'static str, i32>,
    notes: HashMap<&'static str, String>,
}

thread_local! {
    static PHASE: RefCell<Phase> = const { RefCell::new(Phase::Idle) };
    /// Bumped by each Start: a timer from an earlier run does nothing.
    static RUN: RefCell<u32> = const { RefCell::new(0) };
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    /// A prompt check is waiting for this connection; reset when the buds disconnect.
    static ASK_PENDING: RefCell<bool> = const { RefCell::new(false) };
}

fn phase() -> Phase { PHASE.with(|p| *p.borrow()) }
fn set_phase(p: Phase) { PHASE.with(|x| *x.borrow_mut() = p); }

fn shown(a: &App) -> Vec<&'static Feature> { FEATURES.iter().filter(|f| (f.3)(a)).collect() }

fn model_id(a: &App) -> Option<String> { a.snap.model.and_then(|m| m["id"].as_str()).map(String::from) }

/// The features an owner confirmed on the current model (`models.json` `confirmed`).
fn confirmed(a: &App) -> HashSet<String> {
    a.snap.model.and_then(|m| m["confirmed"].as_array())
        .map(|l| l.iter().filter_map(|k| k.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn asked() -> Vec<String> {
    load_settings()["test_asked"].as_array().map(|l| l.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

pub fn setup(main: &MainWindow) {
    let g = main.global::<G>();
    g.on_open(|| with_app(render));
    g.on_start(|| with_app(start));
    g.on_mark(|i, mark| with_app(|a| {
        let Some(f) = shown(a).get(i as usize).copied() else { return };
        STATE.with(|s| if let Some(s) = s.borrow_mut().as_mut() { s.marks.insert(f.0, mark); });
        render(a);
    }));
    g.on_set_note(|i, text| with_app(|a| {
        let Some(f) = shown(a).get(i as usize).copied() else { return };
        STATE.with(|s| if let Some(s) = s.borrow_mut().as_mut() { s.notes.insert(f.0, text.trim().to_string()); });
        paint_send(a);
    }));
    g.on_show_preview(|| with_app(|a| {
        if let Some(d) = draft(a) { a.main.global::<G>().set_preview(crate::report::preview(&d).into()); }
    }));
    g.on_send(|| with_app(send));
}

/// Clears the log, drops the link and connects again; [apply] follows the status from there.
fn start(a: &mut App) {
    if phase() != Phase::Idle { return; }
    let run = RUN.with(|r| { *r.borrow_mut() += 1; *r.borrow() });
    LOG.lock().unwrap().clear();
    a.log_shown = 0;
    crate::devtools::refresh(a);
    let name = a.snap.model.and_then(|m| m["name"].as_str()).unwrap_or("unknown model");
    crate::session::log("NOTE", Vec::new(), Some(format!("MODEL TEST: start, {name}")));
    STATE.with(|s| *s.borrow_mut() = None);
    let g = a.main.global::<G>();
    g.set_busy(true);
    g.set_started(false);
    g.set_preview("".into());
    g.set_status(t(&a.tr, "test_reconnecting").into());
    if a.snap.status == Status::Off {
        set_phase(Phase::Connecting);
        let _ = a.tx.send(Cmd::Connect);
    } else {
        set_phase(Phase::Dropping);
        let _ = a.tx.send(Cmd::Disconnect);
    }
    slint::Timer::single_shot(CONNECT_TIMEOUT, move || with_app(|a| {
        if RUN.with(|r| *r.borrow()) == run && matches!(phase(), Phase::Dropping | Phase::Connecting) { fail(a); }
    }));
}

fn fail(a: &mut App) {
    set_phase(Phase::Idle);
    let g = a.main.global::<G>();
    g.set_busy(false);
    g.set_status(t(&a.tr, "test_failed").into());
}

/// Each new state from the session: drives Start, and asks the launch prompt once the buds are in.
pub fn apply(a: &mut App) {
    let on = a.snap.status == Status::On;
    match phase() {
        Phase::Dropping if a.snap.status == Status::Off => {
            set_phase(Phase::Connecting);
            let tx = a.tx.clone();
            slint::Timer::single_shot(RECONNECT_GAP, move || { let _ = tx.send(Cmd::Connect); });
        }
        Phase::Connecting if on => {
            set_phase(Phase::Reading);
            let run = RUN.with(|r| *r.borrow());
            slint::Timer::single_shot(HANDSHAKE, move || with_app(|a| {
                if RUN.with(|r| *r.borrow()) == run && phase() == Phase::Reading { finish(a); }
            }));
        }
        _ => {}
    }
    if !on { ASK_PENDING.with(|p| *p.borrow_mut() = false); return; }
    if ASK_PENDING.with(|p| p.replace(true)) { return; }
    slint::Timer::single_shot(ASK_DELAY, || with_app(maybe_ask));
}

/// The check from the fresh connection's state, then the feature rows.
fn finish(a: &mut App) {
    set_phase(Phase::Idle);
    let Some(model) = model_id(a) else { return fail(a) };
    let s = &a.snap;
    let checks = [s.battery.iter().any(Option::is_some), s.caps.read(), !s.features.is_empty()];
    crate::session::log("NOTE", Vec::new(), Some(format!("MODEL TEST: check battery={} features={} states={}",
        checks[0], checks[1], checks[2])));
    STATE.with(|st| *st.borrow_mut() = Some(State { model, checks, marks: HashMap::new(), notes: HashMap::new() }));
    let g = a.main.global::<G>();
    g.set_busy(false);
    g.set_status("".into());
    render(a);
}

/// The page from the saved state; a state for another model is dropped.
fn render(a: &mut App) {
    let g = a.main.global::<G>();
    let id = model_id(a);
    STATE.with(|s| if s.borrow().as_ref().is_some_and(|s| Some(&s.model) != id.as_ref()) { *s.borrow_mut() = None; });
    let rows: Option<(Vec<TestCheck>, Vec<TestRow>)> = STATE.with(|s| s.borrow().as_ref().map(|s| {
        let checks = CHECKS.iter().zip(s.checks).map(|((_, label, _), ok)| TestCheck {
            title: t(&a.tr, label).into(),
            sub: t(&a.tr, if ok { "test_answered" } else { "test_no_answer" }).into(),
        }).collect();
        let rows = shown(a).into_iter().map(|f| {
            let mark = s.marks.get(f.0).copied().unwrap_or(0);
            let note = s.notes.get(f.0).cloned().unwrap_or_default();
            let sub = match mark {
                WORKS => t(&a.tr, "test_works").to_string(),
                BROKEN => t(&a.tr, "test_broken").to_string(),
                SKIP => t(&a.tr, "test_skipped").to_string(),
                _ => t(&a.tr, "test_not_marked").to_string(),
            };
            TestRow { title: t(&a.tr, f.1).into(), sub: sub.into(), mark, note: note.into() }
        }).collect();
        (checks, rows)
    }));
    g.set_started(rows.is_some());
    if let Some((checks, rows)) = rows {
        g.set_checks(ModelRc::new(VecModel::from(checks)));
        let current = g.get_rows();
        // In place: a new model would rebuild the note fields and drop the text being typed.
        if current.row_count() == rows.len() {
            for (i, r) in rows.into_iter().enumerate() {
                if current.row_data(i).is_some_and(|c| c.mark != r.mark || c.title != r.title) { current.set_row_data(i, r); }
            }
        } else {
            g.set_rows(ModelRc::new(VecModel::from(rows)));
        }
    }
    paint_send(a);
}

/// Every feature shown has a mark, and every "does not work" has a note.
fn complete(a: &App, s: &State) -> bool {
    shown(a).iter().all(|f| match s.marks.get(f.0) {
        None | Some(0) => false,
        Some(&BROKEN) => s.notes.get(f.0).is_some_and(|n| !n.is_empty()),
        _ => true,
    })
}

/// Send needs every feature marked and a log with frames from the buds.
fn ready(a: &App) -> bool {
    STATE.with(|s| s.borrow().as_ref().is_some_and(|s| complete(a, s))) && LOG.lock().unwrap().iter().any(|l| l.dir == "RX")
}

fn paint_send(a: &App) {
    let g = a.main.global::<G>();
    if g.get_busy() { return; }
    g.set_note(if ready(a) { "".into() } else { t(&a.tr, "test_mark_all").into() });
}

/// The start of the machine-readable line. `scripts/import-model-test.py` finds the JSON after it.
const JSON_PREFIX: &str = "MODEL TEST JSON ";

/// The row key of a feature key, where the two differ (the phone's `ModelTest.ROW_KEY`). A feature without a row
/// (battery) is absent.
const ROW_KEY: &[(&str, &str)] = &[
    ("wind", "wind"), ("game", "game"), ("hires", "hires"), ("spatial", "spatial"), ("golden", "golden"),
    ("eq", "eq"), ("dual", "dual"), ("anc", "anc"), ("gestures", "controls"), ("wear", "wear"), ("find", "find"),
];

/// The rows this app shows for the features the test lists, and the Earbud settings row it always shows. The gates
/// are the ones the pages use ([FEATURES]), so the app has one rule set.
fn rows(a: &App) -> BTreeSet<&'static str> {
    let mut out = BTreeSet::from(["earbuds"]);
    out.extend(shown(a).iter().filter_map(|f| ROW_KEY.iter().find(|(k, _)| *k == f.0).map(|(_, row)| *row)));
    out
}

/// What the buds' answers do not match (the phone's `ModelTest.Diff`):
///  - `missing`: a switch the app asked for in `0x010D`, and the reply did not list;
///  - `extra`: a switch in the reply that the app did not ask for;
///  - `hidden`: a feature the person marked `works`, but the app shows no row for it. Then a row rule hides
///    something that works.
struct Diff {
    missing: Vec<u8>,
    extra: Vec<u8>,
    hidden: Vec<&'static str>,
}

impl Diff {
    fn empty(&self) -> bool { self.missing.is_empty() && self.extra.is_empty() && self.hidden.is_empty() }

    /// One line for the report's description, in English for the developer.
    fn text(&self) -> String {
        let ids = |v: &[u8]| v.iter().map(|id| format!("0x{id:02X}")).collect::<Vec<_>>().join(" ");
        let mut parts = vec![];
        if !self.missing.is_empty() { parts.push(format!("asked, no answer: {}", ids(&self.missing))); }
        if !self.extra.is_empty() { parts.push(format!("answered, not asked: {}", ids(&self.extra))); }
        if !self.hidden.is_empty() {
            let names: Vec<&str> = FEATURES.iter().filter(|f| self.hidden.contains(&f.0)).map(|f| f.2).collect();
            parts.push(format!("marked works, no row: {}", names.join(", ")));
        }
        parts.join("; ")
    }
}

/// The diagnosis from plain values, so a unit test can call it ([diff_state] passes the live state). `reply` is
/// None when nothing was read yet.
/// The two game-mode ids are alternatives (the phone's `ModelTest.otherGameId`): when the app asked both and the
/// reply holds one, the other is no missing answer. Both ids must be in the ask list for this to apply.
fn other_game_id(answered: &HashSet<u8>, asked: &[u8], id: u8) -> bool {
    let alt = match id { 0x06 => 0x28, 0x28 => 0x06, _ => return false };
    answered.contains(&alt) && asked.contains(&alt)
}

fn diff(asked: &[u8], reply: Option<&[(u8, u8)]>, rows: &BTreeSet<&'static str>,
        marks: &HashMap<&'static str, i32>) -> Diff {
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

/// [diff] from the state of the test in progress: the ids this app asks ([status_query]), the last status reply,
/// the rows shown, and the marks.
fn diff_state(a: &App, s: &State) -> Diff {
    let mut asked = status_query(a.snap.model);
    // The count comes first.
    if !asked.is_empty() { asked.remove(0); }
    let reply = (!a.snap.features.is_empty()).then_some(a.snap.features.as_slice());
    diff(&asked, reply, &rows(a), &s.marks)
}

/// The word a mark has in the JSON and the report, as the phone writes it.
fn mark_name(mark: i32) -> Option<&'static str> {
    Some(match mark { WORKS => "works", BROKEN => "broken", SKIP => "skip", _ => return None })
}

/// The one machine-readable line of this test, for the packet log: the model, the firmware, the two replies that
/// decide the rows, the rows the app showed and expected, the marks, and the commands the app did not decode. A
/// report then needs no heuristics: `scripts/import-model-test.py` reads this line and makes a row fixture from it.
/// Same keys and formats as the phone's `ModelTest.jsonLine`.
fn json_line_from(id: &str, name: &str, fw: &str, caps: Option<&[u16]>, status: &[(u8, u8)],
        shown: &[&'static str], expected: &BTreeSet<&'static str>, marks: &HashMap<&'static str, i32>,
        unknown_rx: &BTreeSet<u16>) -> String {
    let mut caps = caps.unwrap_or(&[]).to_vec();
    caps.sort_unstable();
    caps.dedup();
    let caps = caps.iter().map(|c| format!("{c:04X}")).collect::<Vec<_>>().join(" ");
    let status: serde_json::Map<String, serde_json::Value> =
        status.iter().map(|(id, v)| (format!("{id:02X}"), (*v).into())).collect();
    let marks: serde_json::Map<String, serde_json::Value> = shown.iter()
        .filter_map(|k| marks.get(k).and_then(|m| mark_name(*m)).map(|n| (k.to_string(), n.into())))
        .collect();
    let json = serde_json::json!({
        "id": id,
        "name": name,
        "fw": fw,
        "caps": caps,
        "status": status,
        "shown": shown,
        "expected": expected.iter().collect::<Vec<_>>(),
        "marks": marks,
        "unknownRx": unknown_rx.iter().map(|c| format!("0x{c:04X}")).collect::<Vec<_>>(),
    });
    format!("{JSON_PREFIX}{json}")
}

/// [json_line_from] from the live state: the product id, the model, the firmware, the two replies, the features shown, the
/// rows, the marks, and the RX frames in the log that nothing decoded.
fn json_line(a: &App, s: &State) -> String {
    let keys: Vec<&'static str> = shown(a).iter().map(|f| f.0).collect();
    let name = a.snap.model.and_then(|m| m["name"].as_str()).unwrap_or("unknown model");
    let id = load_settings()["product_id"].as_str().unwrap_or_default().to_string();
    json_line_from(&id, name, a.snap.firmware.as_deref().unwrap_or(""), a.snap.caps.0.as_deref(),
        &a.snap.features, &keys, &rows(a), &s.marks, &unknown_rx())
}

/// The command ids of the RX frames the app did not decode: the phone's `UNATTR RX:` lines. A `0x0204` push stays
/// out, as on the phone: its sub-type is not a command.
fn unknown_rx() -> BTreeSet<u16> {
    LOG.lock().unwrap().iter()
        .filter(|l| l.dir == "RX" && l.human.is_none())
        .map(|l| cmd_of(&l.bytes))
        .filter(|c| *c != EVT_PUSH)
        .collect()
}

/// The report's description, in English for the developer: the check, the diagnosis, then one line per feature.
fn description(a: &App, s: &State) -> String {
    let check: Vec<String> = CHECKS.iter().zip(s.checks)
        .map(|((_, _, name), ok)| format!("{name} {}", if ok { "answered" } else { "no answer" })).collect();
    let mut lines = vec![
        format!("Model test ({})", if cfg!(windows) { "Windows" } else { "Linux" }),
        format!("Automatic check: {}", check.join(", ")),
    ];
    let diagnosis = diff_state(a, s);
    if !diagnosis.empty() { lines.push(format!("Diagnosis: {}", diagnosis.text())); }
    for f in shown(a) {
        lines.push(match s.marks.get(f.0) {
            Some(&WORKS) => format!("{}: works", f.2),
            Some(&BROKEN) => format!("{}: does not work: {}", f.2, s.notes.get(f.0).map_or("", |n| n.as_str())),
            _ => format!("{}: skipped", f.2),
        });
    }
    lines.join("\n")
}

fn draft(a: &App) -> Option<crate::report::Draft> {
    let model = a.snap.model.and_then(|m| m["name"].as_str())?.to_string();
    let description = STATE.with(|s| s.borrow().as_ref().map(|s| description(a, s)))?;
    Some(crate::report::Draft { model, categories: vec!["Test"], description, log: Some(crate::report::log_text(a)), crash: false })
}

fn send(a: &mut App) {
    if !ready(a) { return paint_send(a); }
    // The machine-readable line goes in first, so the log this report sends carries it.
    STATE.with(|st| if let Some(s) = st.borrow().as_ref() {
        crate::session::log("NOTE", Vec::new(), Some(json_line(a, s)));
    });
    let Some(d) = draft(a) else { return };
    let g = a.main.global::<G>();
    g.set_busy(true);
    g.set_status(t(&a.tr, "problem_sending").into());
    let weak = a.main.as_weak();
    std::thread::spawn(move || {
        let (ok, refused) = crate::report::send(d);
        let _ = weak.upgrade_in_event_loop(move |m| {
            let g = m.global::<G>();
            g.set_busy(false);
            if ok {
                STATE.with(|s| *s.borrow_mut() = None);
                g.set_started(false);
                g.set_preview("".into());
                g.set_status("Feedback sent. Thank you!".into());
            } else {
                g.set_status(refused.unwrap_or_else(|| "Could not send the feedback. Check the internet connection and try again.".into()).into());
            }
        });
    });
}

/// Once per model: connected buds with a feature shown that no owner confirmed. Only on Overview, with the window
/// open, after no crash report this start, and never during a test.
fn maybe_ask(a: &mut App) {
    if a.snap.status != Status::On || phase() != Phase::Idle || a.main.get_page() != 0 || !a.main.window().is_visible() { return; }
    if crate::report::crash_shown() { return; }
    let Some(id) = model_id(a) else { return };
    let mut done = asked();
    if done.contains(&id) { return; }
    let ok = confirmed(a);
    if shown(a).iter().all(|f| ok.contains(f.0)) { return; }
    done.push(id);
    save_setting("test_asked", done.into());
    let g = a.main.global::<G>();
    g.set_ask(g.get_ask() + 1);
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

    #[test]
    fn labels_exist() {
        for key in FEATURES.iter().map(|f| f.1).chain(CHECKS.iter().map(|c| c.1)) {
            assert!(crate::strings::KEYS.contains(&key), "{key}");
        }
    }

    /// `confirmed` names only known keys.
    #[test]
    fn confirmed_keys() {
        for m in models() {
            for k in m["confirmed"].as_array().into_iter().flatten() {
                assert!(FEATURES.iter().any(|f| Some(f.0) == k.as_str()), "{}: {k}", m["name"]);
            }
        }
    }

    /// The nine fields `scripts/import-model-test.py` reads, and the formats it parses.
    #[test]
    fn json_line_for_the_import() {
        let marks = HashMap::from([("anc", WORKS), ("eq", BROKEN), ("wind", WORKS)]);
        let line = json_line_from("065414", "OnePlus Buds 4", "1.0.1.108", Some(&[0x0400, 0x010D, 0x0100, 0x010D]),
            &[(0x1B, 0), (0x05, 1)], &["battery", "anc"], &BTreeSet::from(["eq", "anc", "earbuds"]),
            &marks, &BTreeSet::from([0x8400, 0x8123, 0x8400]));
        assert!(!line.contains('\n'), "a line break splits the log line");
        assert_eq!(line, "MODEL TEST JSON {\"caps\":\"0100 010D 0400\",\"expected\":[\"anc\",\"earbuds\",\"eq\"],\
            \"fw\":\"1.0.1.108\",\"id\":\"065414\",\"marks\":{\"anc\":\"works\"},\"name\":\"OnePlus Buds 4\",\
            \"shown\":[\"battery\",\"anc\"],\"status\":{\"05\":1,\"1B\":0},\"unknownRx\":[\"0x8123\",\"0x8400\"]}");
        let v: serde_json::Value = serde_json::from_str(&line[JSON_PREFIX.len()..]).unwrap();
        let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["caps", "expected", "fw", "id", "marks", "name", "shown", "status", "unknownRx"]);
    }

    /// Nothing read yet: the fields stay empty, and no value is invented.
    #[test]
    fn json_line_with_nothing_read() {
        let line = json_line_from("", "unknown model", "", None, &[], &[], &BTreeSet::from(["earbuds"]), &HashMap::new(), &BTreeSet::new());
        assert_eq!(line, "MODEL TEST JSON {\"caps\":\"\",\"expected\":[\"earbuds\"],\"fw\":\"\",\"id\":\"\",\"marks\":{},\
            \"name\":\"unknown model\",\"shown\":[],\"status\":{},\"unknownRx\":[]}");
    }

    /// The diagnosis: one line, the three mismatches in the phone's words.
    #[test]
    fn diagnosis_text() {
        // (asked, reply, rows, marks, the diagnosis)
        let cases: [(&[u8], Option<&[(u8, u8)]>, &[&str], &[(&str, i32)], &str); 6] = [
            // The model asks 0x04 and 0x06 only (its `statusQuery`): 0x28 is a switch it never asked for.
            (&[0x04, 0x06], Some(&[(0x28, 1)]), &["earbuds", "hires"], &[("eq", WORKS)],
                "asked, no answer: 0x04 0x06; answered, not asked: 0x28; marked works, no row: Equalizer"),
            (&[0x05], Some(&[(0x05, 1)]), &["earbuds", "hires"], &[], ""),
            // Nothing read: no missing and no extra.
            (&[0x04, 0x05], None, &["earbuds"], &[], ""),
            // A switch the buds answered that the app did not ask for appears once.
            (&[0x05], Some(&[(0x05, 1), (0x99, 0), (0x99, 1)]), &["earbuds"], &[], "answered, not asked: 0x99"),
            // Names follow the feature list, not the marks.
            (&[0x05], Some(&[(0x05, 1)]), &["earbuds", "eq"], &[("find", WORKS), ("wind", WORKS)],
                "marked works, no row: Wind noise reduction, Find my earbuds"),
            // A feature marked broken without a row is no contradiction.
            (&[0x05], Some(&[(0x05, 1)]), &["earbuds"], &[("eq", BROKEN)], ""),
        ];
        for (asked, reply, row_keys, marks, text) in cases {
            let d = diff(asked, reply, &row_keys.iter().copied().collect(), &marks.iter().copied().collect());
            assert_eq!(d.text(), text);
            assert_eq!(d.empty(), text.is_empty(), "{asked:02X?}");
        }
    }

    /// The three parts, each on its own.
    #[test]
    fn diagnosis_parts() {
        let no_eq = BTreeSet::from(["earbuds", "hires"]);
        let d = diff(&[0x04, 0x06], Some(&[(0x28, 1)]), &no_eq, &HashMap::from([("eq", WORKS)]));
        assert_eq!(d.missing, [0x04, 0x06]);
        assert_eq!(d.extra, [0x28]);
        assert_eq!(d.hidden, ["eq"]);
        let rows = BTreeSet::from(["earbuds", "hires", "eq"]);
        let clean = diff(&[0x28], Some(&[(0x28, 1)]), &rows, &HashMap::from([("eq", WORKS), ("anc", BROKEN)]));
        assert!(clean.empty());
    }

    #[test]
    fn one_game_id_is_no_missing_answer() {
        // The app asks both 0x06 and 0x28; a model answers one of them (phone: report 84).
        let rows = BTreeSet::from(["earbuds"]);
        let d = diff(&[0x06, 0x28], Some(&[(0x06, 1)]), &rows, &HashMap::new());
        assert!(d.missing.is_empty(), "missing {:?}", d.missing);
    }
}
