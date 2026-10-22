//! Controls (`GestureActivity`, `GestureConfig.kt`): per bud, which action each gesture does; the hold's noise
//! cycle; the on-call rows. Rows and options come from the model's HeyMelody `control` / `callControl` lists, the
//! current bindings from the buds' own table (`0x8108`) and hold cycle (`0x810C 02`), read when the page opens.
//! `fn` values are measured (PROTOCOL-LEGACY.md §6): never derive new ones.

use crate::gesture_logic::*;
use crate::types::*;
use crate::session::{Cmd, Snapshot};
use crate::{icons, svg, t, with_app, App, Choice, Controls, ControlsRow, MainWindow, Pick};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::cell::RefCell;

/// Where a pick goes.
#[derive(Clone, Copy)]
enum Target { Row(usize), HoldChoice, Call(usize), BothHold }

#[derive(Default)]
struct State { side: u8, target: Option<Target>, working: Vec<usize> }

thread_local! { static ST: RefCell<State> = RefCell::new(State { side: 1, ..Default::default() }); }

fn st<R>(f: impl FnOnce(&mut State) -> R) -> R { ST.with(|s| f(&mut s.borrow_mut())) }

fn describe(a: &App, actions: &[usize]) -> String {
    if actions.is_empty() { return t(&a.tr, "gesture_not_set").into(); }
    actions.iter().map(|x| t(&a.tr, if ACTIONS[*x].1.is_empty() { ACTIONS[*x].0 } else { ACTIONS[*x].1 })).collect::<Vec<_>>().join(", ")
}

fn send(a: &App, c: Cmd) { let _ = a.tx.send(c); }

fn write(a: &App, m: &Model, side: u8, g: usize, actions: &[usize]) {
    for c in write_cmds(&a.snap, m, side, g, actions) { send(a, c); }
}

pub fn setup(main: &MainWindow) {
    let c = main.global::<Controls>();
    c.on_opened(|| with_app(|a| send(a, Cmd::ControlsReads(Model::of(&a.snap).hold_types()))));
    c.on_set_side(|side| with_app(|a| { st(|s| s.side = side as u8 + 1); apply(a); }));
    // 1 a single-choice list, 2 the per-bud hold's choices, 3 the hold's noise modes; 0 nothing to open.
    c.on_open_row(|i| with_app_ret(|a| {
        let m = Model::of(&a.snap);
        if m.both_hold && i as usize == m.rows.len() {
            st(|s| s.target = Some(Target::BothHold));
            let c = a.main.global::<Controls>();
            c.set_dialog_title(t(&a.tr, "gesture_both_hold").into());
            c.set_choices(ModelRc::new(VecModel::from(vec![
                Choice { value: 0, label: t(&a.tr, "gesture_action_none").into() },
                Choice { value: 1, label: t(&a.tr, "gesture_action_game").into() },
            ])));
            c.set_choice_current(both_hold_on(&a.snap).map_or(-2, |on| on as i32));
            return 1;
        }
        let Some(&(g, _)) = m.rows.get(i as usize) else { return 0 };
        let side = st(|s| s.side);
        let cur = current(&a.snap, &m, side, g);
        let c = a.main.global::<Controls>();
        c.set_dialog_title(t(&a.tr, GESTURES[g].0).into());
        if g == TAP_HOLD && m.per_bud {
            st(|s| s.target = Some(Target::HoldChoice));
            let noise = cur.as_ref().is_some_and(|c| c.iter().any(|x| ACTIONS[*x].4 >= 0));
            let items: Vec<Choice> = m.hold_choices.iter().filter_map(|b| if *b == HOLD_NOISE {
                Some(Choice { value: -1, label: t(&a.tr, "anc_section").into() })
            } else {
                ACTIONS.iter().position(|x| x.3 == *b).map(|x| Choice { value: x as i32, label: t(&a.tr, ACTIONS[x].0).into() })
            }).collect();
            let selected = if noise { -1 } else { cur.and_then(|c| (c.len() == 1).then(|| c[0] as i32)).unwrap_or(-2) };
            c.set_choices(ModelRc::new(VecModel::from(items)));
            c.set_choice_current(selected);
            return 2;
        }
        if g == TAP_HOLD { open_noise(a, &m, cur.unwrap_or_default()); return 3; }
        st(|s| s.target = Some(Target::Row(g)));
        c.set_choices(ModelRc::new(VecModel::from(m.options(g).iter()
            .map(|x| Choice { value: *x as i32, label: t(&a.tr, ACTIONS[*x].0).into() }).collect::<Vec<_>>())));
        c.set_choice_current(cur.and_then(|c| c.first().map(|x| *x as i32)).unwrap_or(-2));
        1
    }));
    c.on_open_call(|i| with_app(|a| {
        let m = Model::of(&a.snap);
        let Some(&k) = m.on_call.get(i as usize) else { return };
        st(|s| s.target = Some(Target::Call(k)));
        let c = a.main.global::<Controls>();
        c.set_dialog_title(t(&a.tr, ON_CALL[k].0).into());
        c.set_choices(ModelRc::new(VecModel::from(vec![
            Choice { value: 0, label: t(&a.tr, "gesture_action_none").into() },
            Choice { value: 1, label: t(&a.tr, ON_CALL[k].1).into() },
        ])));
        c.set_choice_current(call_on(&a.snap, k).map_or(-2, |on| on as i32));
    }));
    // Returns 3 when the noise modes open next.
    c.on_picked(|v| with_app_ret(|a| {
        let m = Model::of(&a.snap);
        let side = st(|s| s.side);
        match st(|s| s.target) {
            Some(Target::Row(g)) => write(a, &m, side, g, &[v as usize]),
            Some(Target::HoldChoice) if v == -1 => {
                let cur = current(&a.snap, &m, side, TAP_HOLD).unwrap_or_default();
                open_noise(a, &m, cur.into_iter().filter(|x| ACTIONS[*x].4 >= 0).collect());
                return 3;
            }
            Some(Target::HoldChoice) => write(a, &m, side, TAP_HOLD, &[v as usize]),
            Some(Target::Call(k)) => send(a, Cmd::OnCall(BUTTON_ON_CALL, ON_CALL[k].2, if v == 1 { ON_CALL[k].3 } else { 0 })),
            Some(Target::BothHold) => send(a, Cmd::OnCall(BUTTON_PRIMARY, 4, if v == 1 { 0x11 } else { 0 })),
            None => {}
        }
        0
    }));
    // Never below the model's minimum, as HeyMelody.
    c.on_toggle(|v| with_app(|a| {
        let m = Model::of(&a.snap);
        st(|s| {
            let v = v as usize;
            if s.working.contains(&v) { if s.working.len() > m.hold_min { s.working.retain(|x| *x != v); } } else { s.working.push(v); }
        });
        paint_noise(a, &m);
    }));
    // In the list's order, so the cycle matches what was shown; below the minimum (a never-set hold) nothing is sent.
    c.on_done(|| with_app(|a| {
        let m = Model::of(&a.snap);
        let working = st(|s| s.working.clone());
        if working.len() >= m.hold_min {
            let ordered: Vec<usize> = m.options(TAP_HOLD).iter().copied().filter(|x| working.contains(x)).collect();
            write(a, &m, st(|s| s.side), TAP_HOLD, &ordered);
        }
    }));
}

/// [with_app] for a callback that returns a value.
fn with_app_ret(f: impl FnOnce(&mut App) -> i32) -> i32 {
    let mut out = 0;
    with_app(|a| out = f(a));
    out
}

fn open_noise(a: &App, m: &Model, cur: Vec<usize>) {
    st(|s| s.working = cur);
    a.main.global::<Controls>().set_dialog_title(t(&a.tr, "gesture_hold").into());
    paint_noise(a, m);
}

fn paint_noise(a: &App, m: &Model) {
    let working = st(|s| s.working.clone());
    let c = a.main.global::<Controls>();
    c.set_picks(ModelRc::new(VecModel::from(m.options(TAP_HOLD).iter()
        .map(|x| Pick { value: *x as i32, label: t(&a.tr, ACTIONS[*x].0).into(), on: working.contains(x) }).collect::<Vec<_>>())));
    c.set_pick_message(if working.len() == 1 && m.hold_min == 1 { t(&a.tr, "gesture_hold_rule").into() } else { "".into() });
}

/// Buds with a gesture table to edit (`EarbudSettingsActivity`'s gestures row).
pub fn available(s: &Snapshot) -> bool { s.caps.supports(CMD_SET_KEY_FUNCTION) && !Model::of(s).rows.is_empty() }

pub fn apply(a: &App) {
    let s = &a.snap;
    let m = Model::of(s);
    if m.one_button { st(|x| x.side = 1); }
    let side = st(|x| x.side);
    let mut rows: Vec<ControlsRow> = m.rows.iter().map(|(g, _)| ControlsRow {
        icon: svg(GESTURES[*g].4), title: t(&a.tr, GESTURES[*g].0).into(),
        value: current(s, &m, side, *g).map_or("—".into(), |c| describe(a, &c)).into(),
    }).collect();
    if m.both_hold {
        rows.push(ControlsRow { icon: svg(icons::HOLD), title: t(&a.tr, "gesture_both_hold").into(),
            value: both_hold_on(s).map_or("—", |on| t(&a.tr, if on { "gesture_action_game" } else { "gesture_action_none" })).into() });
    }
    let calls: Vec<ControlsRow> = m.on_call.iter().map(|k| ControlsRow {
        icon: svg(ON_CALL[*k].5), title: t(&a.tr, ON_CALL[*k].0).into(),
        value: call_on(s, *k).map_or("—", |on| t(&a.tr, if on { ON_CALL[*k].1 } else { "gesture_action_none" })).into(),
    }).collect();
    let c = a.main.global::<Controls>();
    crate::update_rows(c.get_rows(), rows, |m| c.set_rows(m));
    crate::update_rows(c.get_calls(), calls, |m| c.set_calls(m));
    c.set_side(side as i32 - 1);
    c.set_one_button(m.one_button);
}
