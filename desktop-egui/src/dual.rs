//! Dual connection (`DualDeviceActivity`): the switch, the devices the buds list and, on models with
//! HeyMelody's device manager, connect / disconnect and the preferred device.

use crate::glass::{self, white, Choice, Trailing};
use crate::session::{Cmd, Snapshot};
use crate::types::{PairedDevice, CMD_MULTI_CONNECT, FEATURE_DUAL};
use crate::{t, App};
use eframe::egui::{self, Align2, Pos2, Rect, Ui, Vec2};

const ROW: f32 = 72.0;
const GAP: f32 = 10.0;
const MAX_W: f32 = 760.0;

#[derive(Default)]
pub struct State {
    /// The device the connect / disconnect question is about (index into the shown rows).
    ask: Option<usize>,
    choosing: bool,
    pub add: bool,
}

/// A device-manager function of this model (`multiConnect`), on buds that take `0x0429`.
fn has(s: &Snapshot, f: &str) -> bool {
    s.model.and_then(|m| m["dualDevice"].as_array()).is_some_and(|l| l.iter().any(|x| x == f))
        && (s.caps.supports(CMD_MULTI_CONNECT) || s.manual)
}

/// The rows shown: only connected devices, every device with the device manager.
fn shown(s: &Snapshot) -> Vec<&PairedDevice> {
    let manages = has(s, "connectDisconnectDevice");
    s.devices.iter().filter(|d| d.connected || manages).collect()
}

/// The page opened: the reads the phone's screen does then.
pub fn opened(app: &App) {
    app.send(Cmd::DualReads(has(&app.snap, "setPriorityDevice")));
}

pub fn show(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    let s = app.snap.clone();
    let tr = app.tr.clone();
    let w = body.width().min(MAX_W);
    let x0 = body.left();
    let mut y = body.top();
    let on = s.features.iter().any(|f| f.0 == FEATURE_DUAL && f.1 == 1);
    let manages = has(&s, "connectDisconnectDevice");
    let connected = s.status == crate::session::Status::On;

    let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW));
    glass::flat_at(ui, card, 28.0, 1.0);
    let (_, toggled) = glass::row_at(ui, "dual", card, crate::icons::DEVICES, t(&tr, "dual_title"), t(&tr, "dual_switch_sub"), Trailing::Switch(on), connected);
    if toggled { app.send(Cmd::Dual(!on)); }
    y += ROW + 18.0;

    let rows = shown(&s);
    if !rows.is_empty() {
        glass::text(ui, Pos2::new(x0 + 6.0, y + 14.0), Align2::LEFT_CENTER, t(&tr, if manages { "dual_section_all" } else { "dual_section_devices" }), 14.0, 0.72);
        y += 34.0;
        let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, (ROW + GAP) * rows.len() as f32 - GAP));
        for (i, d) in rows.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(x0, y + (ROW + GAP) * i as f32), Vec2::new(w, ROW));
            glass::flat_at(ui, r, 28.0, 1.0);
            let state = t(&tr, if d.this_device { "dual_connected_this" } else if d.connected { "dual_connected" } else { "dual_not_connected" });
            let can = manages && !d.this_device;
            let trailing = if can { Trailing::Value(t(&tr, if d.connected { "conn_action_disconnect" } else { "conn_action_connect" }).into()) } else { Trailing::None };
            let (clicked, _) = glass::row_at(ui, ("dev", i), r, crate::icons::DEVICES, &d.name, state, trailing, true);
            if clicked && can { app.dual.ask = Some(i); }
        }
        y += card.height() + 18.0;
    }

    if manages {
        let r = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 56.0));
        glass::flat_at(ui, r, 16.0, 1.0);
        let hit = ui.interact(r, ui.scope_id().with("dual-add"), egui::Sense::click());
        glass::text(ui, Pos2::new(r.left() + 53.7, r.center().y), Align2::LEFT_CENTER, t(&tr, "dual_add_title"), 16.0, 1.0);
        glass::icon_at(ui, crate::icons::PLUS, Rect::from_center_size(Pos2::new(r.left() + 16.0 + 11.0, r.center().y), Vec2::splat(22.0)), white(0.85));
        glass::icon_at(ui, crate::icons::CHEVRON_RIGHT, Rect::from_center_size(Pos2::new(r.right() - 16.0 - 11.0, r.center().y), Vec2::splat(22.0)), white(0.7));
        if hit.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
        if hit.clicked() { app.dual.add = true; }
        y += 56.0 + 18.0;
    }

    if app.dual.add {
        let mut decision = None;
        let open = crate::settings::sheets::sheet(ui.ctx(), "dual-add", t(&tr, "dual_add_title"), crate::settings::sheets::Title::Centre,
            Vec2::new(crate::settings::sheets::WIDTH, 224.8), |ui, inner| {
                let msg = Rect::from_min_max(inner.min + Vec2::new(14.0, 4.0), Pos2::new(inner.right() - 14.0, inner.bottom() - 66.0));
                crate::phone::paragraph(ui, msg, t(&tr, "dual_add_message"), 14.0, white(0.80), false);
                decision = crate::settings::sheets::actions(ui, inner, "", t(&tr, "dual_add_ok"));
            });
        if decision.is_some() || !open { app.dual.add = false; }
    }

    if has(&s, "setPriorityDevice") {
        let auto = t(&tr, "dual_preferred_auto").to_string();
        let idx = s.preferred.and_then(|m| s.devices.iter().position(|d| d.mac == m));
        let value = match (s.preferred, idx) {
            (None, _) => auto.clone(),
            (Some(_), Some(i)) => s.devices[i].name.clone(),
            (Some(m), None) => m.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":"),
        };
        let card = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, ROW));
        glass::flat_at(ui, card, 28.0, 1.0);
        let (clicked, _) = glass::row_at(ui, "pref", card, crate::icons::DEVICES, t(&tr, "dual_preferred"), t(&tr, "dual_preferred_sub"), Trailing::Value(value), connected);
        if clicked { app.dual.choosing = true; }
        y += ROW + 18.0;
        if app.dual.choosing {
            let options: Vec<String> = std::iter::once(auto).chain(s.devices.iter().map(|d| d.name.clone())).collect();
            match glass::choice_dialog(ui.ctx(), "pref-dialog", t(&tr, "dual_preferred"), &options, idx.map_or(0, |i| i + 1)) {
                Choice::Pick(v) => {
                    app.send(Cmd::Preferred(if v == 0 { None } else { s.devices.get(v - 1).map(|d| d.mac) }));
                    app.dual.choosing = false;
                }
                Choice::Close => app.dual.choosing = false,
                Choice::Open => {}
            }
        }
    }

    if let Some(i) = app.dual.ask {
        let Some(d) = shown(&s).get(i).copied().cloned() else { app.dual.ask = None; return y };
        let q = t(&tr, if d.connected { "dual_disconnect_q" } else { "dual_connect_q" }).replace("%1$s", &d.name);
        let action = t(&tr, if d.connected { "conn_action_disconnect" } else { "conn_action_connect" });
        let mut decision = None;
        let open = glass::dialog(ui.ctx(), "dual-ask", &d.name, Vec2::new(420.0, 200.0), |ui| {
            ui.add(egui::Label::new(egui::RichText::new(&q).size(15.0).color(white(0.9))).wrap());
            decision = glass::dialog_buttons(ui, t(&tr, "dialog_cancel"), action);
        });
        match decision {
            Some(true) => { app.send(Cmd::ConnectDevice(d.mac, !d.connected)); app.dual.ask = None; }
            Some(false) => app.dual.ask = None,
            None => if !open { app.dual.ask = None },
        }
    }
    y
}
