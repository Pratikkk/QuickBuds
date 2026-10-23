//! Controls (`GestureActivity`): per bud, which action each gesture does; the hold's noise cycle; the on-call rows.
//! The model, the current bindings and the writes are `gesture_logic`, shared with the Slint app.

use crate::gesture_logic::*;
use crate::glass::{self, white, Choice, Trailing};
use crate::session::{Cmd, Snapshot, Status};
use crate::types::HOLD_TYPE_SHARED;
use crate::{icons, t, App};
use eframe::egui::{self, Align2, Pos2, Rect, Ui, Vec2};

const ROW: f32 = 64.0;
const MAX_W: f32 = 760.0;

/// Where a pick goes.
#[derive(Clone, Copy)]
enum Target { Row(usize), HoldChoice, Call(usize), BothHold, Noise }

pub struct State { side: u8, target: Option<Target>, working: Vec<usize> }

impl Default for State { fn default() -> Self { State { side: 1, target: None, working: vec![] } } }

fn describe(tr: &[String], actions: &[usize]) -> String {
    if actions.is_empty() { return t(tr, "gesture_not_set").into(); }
    actions.iter().map(|x| t(tr, if ACTIONS[*x].1.is_empty() { ACTIONS[*x].0 } else { ACTIONS[*x].1 })).collect::<Vec<_>>().join(", ")
}

/// The page opened: the gesture table and the hold cycles.
pub fn opened(app: &App) {
    app.send(Cmd::ControlsReads(Model::of(&app.snap).hold_types()));
    let _ = HOLD_TYPE_SHARED;
}

/// Buds with a gesture table to edit.
pub fn available(s: &Snapshot) -> bool { s.caps.supports(crate::types::CMD_SET_KEY_FUNCTION) && !Model::of(s).rows.is_empty() }

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    let s = app.snap.clone();
    let tr = app.tr.clone();
    let m = Model::of(&s);
    // Wide window: the tap rows on the left, the on-call rows on the right.
    let two = body.width() >= 900.0;
    let w = if two { ((body.width() - 14.0) / 2.0).floor() } else { body.width().min(MAX_W) };
    let x0 = body.left();
    let mut y = body.top();
    let on = s.status == Status::On;
    if m.one_button { app.gestures.side = 1; }
    let side = app.gestures.side;

    // Left / Right
    if !m.one_button {
        for (i, label) in ["status_left", "status_right"].iter().enumerate() {
            let sw = if two { w } else { w };
            let r = Rect::from_min_size(Pos2::new(x0 + i as f32 * (sw / 2.0 + 4.0), y), Vec2::new(sw / 2.0 - 4.0, 46.0));
            let lit = side == i as u8 + 1;
            if glass::button_at(ui, ("side", i), r, 23.0, lit).clicked() { app.gestures.side = i as u8 + 1; }
            ui.painter().text(r.center(), Align2::CENTER_CENTER, t(&tr, label), egui::FontId::proportional(15.0), if lit { glass::on_accent() } else { glass::white(1.0) });
        }
        y += 46.0 + 18.0;
    }

    // The gesture rows
    let y_top = y;
    let mut n_rows = m.rows.len();
    if m.both_hold { n_rows += 1; }
    if n_rows > 0 {
        let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW * n_rows as f32));
        glass::flat_at(ui, card, 28.0, 1.0);
        for (i, (g, _)) in m.rows.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(x0, y + ROW * i as f32), Vec2::new(w, ROW));
            if i > 0 { glass::divider_at(ui, x0, x0 + w, r.top()); }
            let value = current(&s, &m, side, *g).map_or("—".into(), |c| describe(&tr, &c));
            let (clicked, _) = glass::row_at(ui, ("g", i), r, GESTURES[*g].4, t(&tr, GESTURES[*g].0), "", Trailing::Value(value), on);
            if clicked {
                app.gestures.target = Some(if *g == TAP_HOLD {
                    if m.per_bud { Target::HoldChoice } else { app.gestures.working = current(&s, &m, side, *g).unwrap_or_default(); Target::Noise }
                } else { Target::Row(*g) });
            }
        }
        if m.both_hold {
            let i = m.rows.len();
            let r = Rect::from_min_size(Pos2::new(x0, y + ROW * i as f32), Vec2::new(w, ROW));
            glass::divider_at(ui, x0, x0 + w, r.top());
            let v = both_hold_on(&s).map_or("—", |on| t(&tr, if on { "gesture_action_game" } else { "gesture_action_none" }));
            let (clicked, _) = glass::row_at(ui, "bothhold", r, icons::HOLD, t(&tr, "gesture_both_hold"), "", Trailing::Value(v.into()), on);
            if clicked { app.gestures.target = Some(Target::BothHold); }
        }
        y += card.height() + 18.0;
    }

    // On call
    if !m.on_call.is_empty() {
        let (cx, mut cy) = if two { (x0 + w + 14.0, y_top) } else { (x0, y) };
        glass::text(ui, Pos2::new(cx + 6.0, cy + 14.0), Align2::LEFT_CENTER, t(&tr, "gesture_section_on_call"), 14.0, 0.72);
        cy += 34.0;
        let card = Rect::from_min_size(Pos2::new(cx, cy), Vec2::new(w, ROW * m.on_call.len() as f32));
        glass::flat_at(ui, card, 28.0, 1.0);
        for (i, k) in m.on_call.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(cx, cy + ROW * i as f32), Vec2::new(w, ROW));
            if i > 0 { glass::divider_at(ui, cx, cx + w, r.top()); }
            let v = call_on(&s, *k).map_or("—", |on| t(&tr, if on { ON_CALL[*k].1 } else { "gesture_action_none" }));
            let (clicked, _) = glass::row_at(ui, ("call", i), r, ON_CALL[*k].5, t(&tr, ON_CALL[*k].0), "", Trailing::Value(v.into()), on);
            if clicked { app.gestures.target = Some(Target::Call(*k)); }
        }
        if two { y = y.max(cy + card.height() + 18.0); } else { y += card.height() + 18.0; }
    }
    if n_rows == 0 && m.on_call.is_empty() {
        let r = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 96.0));
        glass::flat_at(ui, r, 28.0, 1.0);
        glass::text(ui, r.center(), Align2::CENTER_CENTER, t(&tr, "eq_not_connected"), 15.0, 0.8);
        y += 96.0;
    }

    dialogs(app, ui, &s, &m, side);
    y
}

/// A single-choice dialog: (value, label) options; Some(value) when one is picked.
fn pick(ui: &mut Ui, id: &str, title: &str, items: &[(i32, String)], current: i32) -> Option<Option<i32>> {
    let labels: Vec<String> = items.iter().map(|x| x.1.clone()).collect();
    let cur = items.iter().position(|x| x.0 == current).unwrap_or(usize::MAX);
    match glass::choice_dialog(ui.ctx(), id, title, &labels, cur) {
        Choice::Pick(i) => Some(Some(items[i].0)),
        Choice::Close => Some(None),
        Choice::Open => None,
    }
}

fn dialogs(app: &mut App, ui: &mut Ui, s: &Snapshot, m: &Model, side: u8) {
    let tr = app.tr.clone();
    let Some(target) = app.gestures.target else { return };
    let mut sends: Vec<Cmd> = vec![];
    let mut next = Some(target);
    match target {
        Target::Row(g) => {
            let items: Vec<(i32, String)> = m.options(g).iter().map(|x| (*x as i32, t(&tr, ACTIONS[*x].0).into())).collect();
            let cur = current(s, m, side, g).and_then(|c| c.first().map(|x| *x as i32)).unwrap_or(-2);
            match pick(ui, "g-row", t(&tr, GESTURES[g].0), &items, cur) {
                Some(Some(v)) => { sends = write_cmds(s, m, side, g, &[v as usize]); next = None; }
                Some(None) => next = None,
                None => {}
            }
        }
        Target::HoldChoice => {
            let cur = current(s, m, side, TAP_HOLD);
            let noise = cur.as_ref().is_some_and(|c| c.iter().any(|x| ACTIONS[*x].4 >= 0));
            let items: Vec<(i32, String)> = m.hold_choices.iter().filter_map(|b| if *b == HOLD_NOISE {
                Some((-1, t(&tr, "anc_section").to_string()))
            } else {
                ACTIONS.iter().position(|x| x.3 == *b).map(|x| (x as i32, t(&tr, ACTIONS[x].0).to_string()))
            }).collect();
            let selected = if noise { -1 } else { cur.and_then(|c| (c.len() == 1).then(|| c[0] as i32)).unwrap_or(-2) };
            match pick(ui, "g-hold", t(&tr, "gesture_hold"), &items, selected) {
                Some(Some(-1)) => {
                    app.gestures.working = current(s, m, side, TAP_HOLD).unwrap_or_default().into_iter().filter(|x| ACTIONS[*x].4 >= 0).collect();
                    next = Some(Target::Noise);
                }
                Some(Some(v)) => { sends = write_cmds(s, m, side, TAP_HOLD, &[v as usize]); next = None; }
                Some(None) => next = None,
                None => {}
            }
        }
        Target::BothHold => {
            let items = vec![(0, t(&tr, "gesture_action_none").to_string()), (1, t(&tr, "gesture_action_game").to_string())];
            match pick(ui, "g-both", t(&tr, "gesture_both_hold"), &items, both_hold_on(s).map_or(-2, |on| on as i32)) {
                Some(Some(v)) => { sends.push(Cmd::OnCall(crate::types::BUTTON_PRIMARY, 4, if v == 1 { 0x11 } else { 0 })); next = None; }
                Some(None) => next = None,
                None => {}
            }
        }
        Target::Call(k) => {
            let items = vec![(0, t(&tr, "gesture_action_none").to_string()), (1, t(&tr, ON_CALL[k].1).to_string())];
            match pick(ui, "g-call", t(&tr, ON_CALL[k].0), &items, call_on(s, k).map_or(-2, |on| on as i32)) {
                Some(Some(v)) => { sends.push(Cmd::OnCall(crate::types::BUTTON_ON_CALL, ON_CALL[k].2, if v == 1 { ON_CALL[k].3 } else { 0 })); next = None; }
                Some(None) => next = None,
                None => {}
            }
        }
        Target::Noise => {
            let opts: Vec<usize> = m.options(TAP_HOLD).to_vec();
            let h = 60.0 + 52.0 * opts.len() as f32 + 96.0;
            let mut working = app.gestures.working.clone();
            let mut decision = None;
            let open = glass::dialog(ui.ctx(), "g-noise", t(&tr, "gesture_hold"), Vec2::new(420.0, h), |ui| {
                let r = ui.max_rect();
                for (i, a) in opts.iter().enumerate() {
                    let row = Rect::from_min_size(Pos2::new(r.left() - 12.0, r.top() + 52.0 * i as f32 - 8.0), Vec2::new(r.width() + 24.0, 52.0));
                    let lit = working.contains(a);
                    if glass::list_item_at(ui, ("noise", i), row, t(&tr, ACTIONS[*a].0), lit, false).clicked() {
                        // Never below the model's minimum, as HeyMelody.
                        if lit { if working.len() > m.hold_min { working.retain(|x| x != a); } } else { working.push(*a); }
                    }
                }
                let msg_y = r.top() + 52.0 * opts.len() as f32 + 8.0;
                if working.len() == 1 && m.hold_min == 1 {
                    glass::text(ui, Pos2::new(r.left(), msg_y + 10.0), Align2::LEFT_CENTER, t(&tr, "gesture_hold_rule"), 12.5, 0.7);
                }
                decision = glass::dialog_buttons(ui, t(&tr, "dialog_cancel"), t(&tr, "eq_save"));
            });
            app.gestures.working = working.clone();
            match decision {
                Some(true) => {
                    // In the list's order, so the cycle matches what was shown; below the minimum nothing is sent.
                    if working.len() >= m.hold_min {
                        let ordered: Vec<usize> = opts.iter().copied().filter(|x| working.contains(x)).collect();
                        sends = write_cmds(s, m, side, TAP_HOLD, &ordered);
                    }
                    next = None;
                }
                Some(false) => next = None,
                None => if !open { next = None },
            }
        }
    }
    app.gestures.target = next;
    for c in sends { app.send(c); }
    let _ = white(1.0);
}
