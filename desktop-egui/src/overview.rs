//! The window: header and the Overview page, on the glass widgets.

use crate::glass::{self, white, Trailing};
use crate::session::{Cmd, Snapshot, Status};
use crate::home_logic;
use crate::types::{self, FEATURE_DUAL, FEATURE_HEARING, FEATURE_HIRES, FEATURE_SPATIAL};
use crate::{icons, t, App, Page};
use eframe::egui::{self, Align2, Pos2, Rect, Ui, Vec2};

const GAP: f32 = 12.0;
const ROW: f32 = 72.0;

/// A fixed snapshot of Buds 4 for screenshots and UI tests (`QB_DEMO=1`): no link, no settings written.
pub fn demo_snapshot() -> Snapshot {
    let model = types::models().iter().find(|m| m["name"].as_str() == Some("OnePlus Buds 4"));
    Snapshot {
        status: Status::On,
        name: "OnePlus Buds 4".into(),
        battery: [Some((80, false)), Some((90, false)), Some((60, false))],
        wear: [3, 3, 0],
        anc: Some("Off".into()),
        modes: types::AncModes::of(model),
        low_latency: Some(false),
        model,
        codec: Some(3),
        devices: vec![
            types::PairedDevice { mac: [1, 2, 3, 4, 5, 6], name: "This computer".into(), connected: true, this_device: true },
            types::PairedDevice { mac: [1, 2, 3, 4, 5, 7], name: "Phone".into(), connected: true, this_device: false },
        ],
        keyfn: Some(vec![[1, 1, 1, 0x01], [2, 1, 1, 0x01], [1, 1, 2, 0x06], [2, 1, 2, 0x05], [1, 1, 3, 0x00], [2, 1, 3, 0x00], [1, 1, 5, 0x07], [2, 1, 5, 0x07], [1, 1, 4, 0x08], [2, 1, 4, 0x08]]),
        hold_masks: vec![(types::HOLD_TYPE_SHARED, 0x0807)],
        alert_volume: Some(6),
        firmware: Some("138.138.105".into()),
        bass_on: Some(true),
        bass_level: Some(2),
        eq_current: Some(21),
        eq_custom: vec![demo_preset(21, "GoodEQ", [2, 1, 0, -1, 2, 3]), demo_preset(22, "Custom1", [0; 6])],
        features: vec![(types::FEATURE_GAME_MODE, 0), (types::FEATURE_GAME_MODE_MAIN, 0), (FEATURE_HIRES, 1), (FEATURE_SPATIAL, 0), (FEATURE_DUAL, 1), (FEATURE_HEARING, 0)],
        ..Default::default()
    }
}

fn demo_preset(id: u8, name: &str, gains: [i8; 6]) -> types::Preset {
    let mut p = types::Preset::new(name, types::EQ_DEFAULT_FREQS.to_vec());
    p.id = id;
    p.gains = gains.to_vec();
    p
}

/// (mode id, icon, index into the level names)
fn level_info(id: &str) -> (&'static str, usize) {
    match id {
        "ANC-Light" => (icons::MODE_ANC_LOW, 0),
        "ANC-Deep" => (icons::MODE_ANC_HIGH, 2),
        "ANC-Smart" => (icons::MODE_ANC_SMART, 3),
        _ => (icons::MODE_ANC_MEDIUM, 1),
    }
}

const LEVEL_NAMES: [&str; 4] = ["anc_mode_low", "anc_mode_medium", "anc_mode_high", "anc_mode_smart"];

/// Without an "in ear" seen, "out" may only mean the model has no sensor: shown as in the ear (the app's rule).
pub fn wear_sensor_seen(s: &Snapshot, demo: bool) -> bool {
    if demo { return true; }
    let Some(id) = s.model.and_then(|m| m["id"].as_str()) else { return true };
    let mut seen = crate::load_settings()["wear_seen"].clone();
    if seen[id] == true { return true; }
    if !s.wear.iter().any(|w| matches!(w, 3 | 7)) { return false; }
    seen[id] = true.into();
    crate::save_setting("wear_seen", seen);
    true
}

/// A strip that moves the window when dragged. The window has a fixed size: no resize edges, no maximize.
fn drag_strip(ui: &mut Ui, id: &str, rect: Rect) {
    let r = ui.interact(rect, ui.scope_id().with(id), egui::Sense::click_and_drag());
    if r.drag_started_by(egui::PointerButton::Primary) { ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag); }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let full = ui.max_rect();
    let content = Rect::from_min_max(Pos2::new(full.left() + 24.0, full.top() + 20.0), Pos2::new(full.right() - 24.0, full.bottom() - 22.0));
    header(app, ui, content);
    let body = Rect::from_min_max(Pos2::new(content.left(), content.top() + 58.0), content.max);
    confirm_dialog(app, ui);
    if app.page != app.shown_page {
        app.shown_page = app.page;
        app.page_since = std::time::Instant::now();
        app.scroll = 0.0;
        if !app.demo {
            match app.page { Page::Dual => crate::dual::opened(app), Page::Earbuds => app.send(Cmd::EarbudReads), Page::Gestures => crate::gestures::opened(app), Page::Hearing => crate::hearing::opened(app), _ => {} }
        }
    }
    if matches!(app.page, Page::Overview | Page::Eq | Page::Dual | Page::Earbuds | Page::Dev | Page::Gestures | Page::Hearing | Page::Settings | Page::Models) {
        // A page scrolls when it is taller than the window: a child ui clipped to the body.
        // A page rises and fades in.
        let t = (app.page_since.elapsed().as_secs_f32() / 0.3).min(1.0);
        let ease = 1.0 - (1.0 - t).powi(3);
        if t < 1.0 { ui.ctx().request_repaint(); }
        let mut page = ui.new_child(egui::UiBuilder::new().max_rect(body));
        page.set_clip_rect(body.intersect(ui.clip_rect()));
        page.set_opacity(ease);
        if page.rect_contains_pointer(body) {
            let dy = page.input(|i| i.smooth_scroll_delta.y);
            if dy != 0.0 { app.scroll = (app.scroll - dy).max(0.0); }
        }
        let shifted = Rect::from_min_max(Pos2::new(body.left(), body.top() - app.scroll + 16.0 * (1.0 - ease)), body.max);
        let bottom = match app.page {
            Page::Eq => crate::eq::show(app, &mut page, shifted),
            Page::Dual => crate::dual::show(app, &mut page, shifted),
            Page::Earbuds => crate::earbuds::show(app, &mut page, shifted),
            Page::Dev => crate::dev::show(app, &mut page, shifted),
            Page::Gestures => crate::gestures::show(app, &mut page, shifted),
            Page::Hearing => crate::hearing::show(app, &mut page, shifted),
            Page::Models => crate::models::show(app, &mut page, shifted),
            Page::Settings => crate::settings::show(app, &mut page, shifted),
            _ => overview(app, &mut page, shifted),
        };
        // A page that shrinks (the EQ editor closes) eases the scroll back instead of snapping.
        let max = (bottom + app.scroll - body.bottom() + 12.0).max(0.0);
        if app.scroll > max {
            let dt = ui.input(|i| i.stable_dt).min(0.05);
            app.scroll += (max - app.scroll) * (1.0 - (-14.0 * dt).exp());
            if app.scroll - max < 0.5 { app.scroll = max; } else { ui.ctx().request_repaint(); }
        }
    } else {
        placeholder(ui, body);
    }
}

fn header(app: &mut App, ui: &mut Ui, c: Rect) {
    let s = &app.snap;
    let title = if s.name.is_empty() { "QuickBuds" } else { s.name.as_str() };
    // The window has no title bar: the header and the top of the sidebar move it.
    drag_strip(ui, "drag-header", Rect::from_min_max(Pos2::new(c.left(), c.top() - 18.0), Pos2::new(c.right(), c.top() + 52.0)));
    // Pages other than the Overview get a back button and their own title.
    let mut tx = c.left() + 4.0;
    let mut title = title;
    if app.page != Page::Overview {
        let r = Rect::from_min_size(Pos2::new(c.left(), c.top()), Vec2::splat(46.0));
        if glass::button_at(ui, "back", r, 23.0, false).clicked() { app.page = if app.page == Page::Models { Page::Earbuds } else { Page::Overview }; }
        let m = r.center() + Vec2::new(1.5, 0.0);
        let st = egui::Stroke::new(2.4, white(1.0));
        ui.painter().line_segment([m + Vec2::new(-3.0, -8.0), m + Vec2::new(-11.0, 0.0)], st);
        ui.painter().line_segment([m + Vec2::new(-11.0, 0.0), m + Vec2::new(-3.0, 8.0)], st);
        ui.painter().line_segment([m + Vec2::new(-11.0, 0.0), m + Vec2::new(8.0, 0.0)], st);
        tx = c.left() + 60.0;
        title = t(&app.tr, match app.page { Page::Eq => "eq_title", Page::Gestures => "gesture_title", Page::Hearing => "row_golden_title", Page::Dual => "dual_title",
            Page::Earbuds | Page::Models => "earbuds_title", Page::Settings => "desktop_nav_app_settings", Page::Dev => "action_dev_tools", Page::Overview => "earbuds_title" });
    }
    glass::text(ui, Pos2::new(tx, c.top() + 23.0), Align2::LEFT_CENTER, title, 30.0, 1.0);
    let mut x = c.right();
    // Our own close button, top right; the other buttons sit left of it.
    {
        x -= 46.0;
        let r = Rect::from_min_size(Pos2::new(x, c.top()), Vec2::splat(46.0));
        x -= 10.0;
        let resp = glass::button_at(ui, "close", r, 23.0, false);
        let k = 8.0;
        let col = white(1.0);
        ui.painter().line_segment([r.center() + Vec2::new(-k, -k), r.center() + Vec2::new(k, k)], egui::Stroke::new(2.2, col));
        ui.painter().line_segment([r.center() + Vec2::new(-k, k), r.center() + Vec2::new(k, -k)], egui::Stroke::new(2.2, col));
        if resp.clicked() { ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close); }
    }
    let icon_btn = |ui: &mut Ui, x: &mut f32, id: &str, svg: &'static str| {
        *x -= 46.0;
        let r = Rect::from_min_size(Pos2::new(*x, c.top()), Vec2::splat(46.0));
        *x -= 10.0;
        glass::icon_button_at(ui, id, r, svg).clicked()
    };
    if icon_btn(ui, &mut x, "settings", icons::SETTINGS_COG) { app.page = Page::Settings; }
    if icon_btn(ui, &mut x, "dev", icons::DEV_TOOLS) { app.page = Page::Dev; }
    let label = match app.snap.status {
        Status::On => t(&app.tr, "conn_action_disconnect"),
        Status::Connecting => t(&app.tr, "conn_connecting"),
        Status::Off => t(&app.tr, "conn_action_connect"),
    };
    let w = 56.0 + label.chars().count() as f32 * 9.0;
    let r = Rect::from_min_size(Pos2::new(x - w, c.top()), Vec2::new(w, 46.0));
    if glass::button_at(ui, "conn", r, 23.0, false).clicked() {
        app.send(if app.snap.status == Status::Off { Cmd::Connect } else { Cmd::Disconnect });
    }
    let dot = if app.snap.status == Status::On { 1.0 } else { 0.45 };
    ui.painter().circle_filled(Pos2::new(r.left() + 24.0, r.center().y), 10.0, white(dot));
    glass::text(ui, Pos2::new(r.left() + 44.0, r.center().y), Align2::LEFT_CENTER, label, 16.0, 1.0);
}

fn placeholder(ui: &mut Ui, body: Rect) {
    let r = Rect::from_min_size(body.min, Vec2::new(body.width(), 120.0));
    glass::flat_at(ui, r, 32.0, 1.0);
    glass::text(ui, r.center(), Align2::CENTER_CENTER, "This page is not ported to egui yet.", 16.0, 0.8);
}

/// `confirm`: the string asked before the write (the buds reconnect).
struct HomeRow { id: &'static str, icon: &'static str, title: String, sub: String, trailing: Trailing, enabled: bool, page: Option<Page>, confirm: Option<&'static str> }

/// Draws the Overview from `body` (already scrolled) and returns the bottom of its content.
fn overview(app: &mut App, ui: &mut Ui, body: Rect) -> f32 {
    let s = app.snap.clone();
    let on = s.status == Status::On;
    let listed = |id: u8| s.features.iter().find(|f| f.0 == id).map(|f| f.1);
    let tr = &app.tr;

    // Rows on the right: the switches the buds list. Hi-Res and 3D audio show their state.
    let mut right: Vec<HomeRow> = vec![];
    if types::has_feature(&s.features, s.caps.game_mode_id()) {
        right.push(HomeRow { id: "game", icon: icons::LOW_LATENCY, title: t(tr, "row_game_title").into(), sub: t(tr, "row_game_sub").into(),
            trailing: Trailing::Switch(s.low_latency.unwrap_or(false)), enabled: on, page: None, confirm: None });
    }
    let hires_on = listed(FEATURE_HIRES) == Some(1);
    let picker = home_logic::codec_picker(&s);
    if listed(FEATURE_HIRES).is_some() || picker {
        let msg = if !picker && home_logic::spatial_on(&s) && home_logic::exclusive(&s) { "codec_msg_hires_drops_spatial" } else { "codec_msg_reconnect" };
        // HeyMelody's codec picker: Hi-Res only with LDAC or LHDC V5.
        let locked = picker && !matches!(s.codec, Some(3 | 8));
        right.push(HomeRow { id: "hires", icon: icons::HIRES, title: t(tr, "row_hires_title").into(),
            sub: t(tr, if hires_on { "row_hires_sub" } else { "row_hires_sub_off" }).into(), trailing: Trailing::Switch(hires_on), enabled: on && !locked, page: None, confirm: Some(msg) });
    }
    if listed(FEATURE_SPATIAL).is_some() || home_logic::spatial_by_type(&s) {
        let msg = if hires_on && home_logic::exclusive(&s) { Some("codec_msg_spatial_drops_hires") } else { None };
        right.push(HomeRow { id: "spatial", icon: icons::SPATIAL, title: t(tr, "row_spatial_title").into(), sub: t(tr, "row_spatial_sub").into(),
            trailing: Trailing::Switch(home_logic::spatial_on(&s)), enabled: on, page: None, confirm: msg });
    }
    let mut left: Vec<HomeRow> = vec![];
    let nav = |id, icon, key: &str, sub: &str, page| HomeRow { id, icon, title: t(tr, key).into(), sub: sub.into(), trailing: Trailing::Chevron, enabled: true, page: Some(page), confirm: None };
    if app.has.eq { left.push(nav("eq", icons::EQUALIZER, "eq_title", t(tr, "row_eq_sub"), Page::Eq)); }
    if app.has.hearing { left.push(nav("hearing", icons::HEARING, "row_golden_title", t(tr, "row_golden_sub"), Page::Hearing)); }
    if app.has.dual { left.push(nav("dual", icons::DEVICES, "dual_title", t(tr, "row_dual_sub"), Page::Dual)); }
    left.push(nav("earbuds", icons::SETTINGS_COG, "earbuds_title", t(tr, "row_earbuds_sub"), Page::Earbuds));

    // Layout A: on the left the battery, the noise control and the Equalizer; on the right the switch and page tiles.
    let mut tiles: Vec<HomeRow> = right;
    let eq_tile = left.iter().position(|r| r.id == "eq").map(|i| left.remove(i));
    if app.has.controls { tiles.push(nav("gestures", icons::GESTURE, "gesture_title", t(tr, "row_gesture_sub"), Page::Gestures)); }
    tiles.extend(left);
    let wide = body.width() >= 900.0;
    let lw = if wide { 520.0_f32.min(body.width() * 0.45).floor() } else { body.width() };
    let h_anc = 112.0;
    let h_batt = ((body.height() - h_anc - GAP * 2.0 - 120.0) * 0.8).clamp(250.0, 330.0);
    let batt = Rect::from_min_size(body.min, Vec2::new(lw, h_batt));
    let anc = Rect::from_min_size(Pos2::new(body.left(), batt.bottom() + GAP), Vec2::new(lw, h_anc));
    battery(app, ui, batt);
    noise(app, ui, anc);
    let mut bottom = anc.bottom();
    let (tx0, ty0, tw) = if wide { (body.left() + lw + GAP, body.top(), body.width() - lw - GAP) } else { (body.left(), anc.bottom() + GAP, body.width()) };
    let mut ty = if wide { ty0 } else { ty0 };
    if let Some(eqr) = eq_tile {
        let h = if wide { (body.bottom() - anc.bottom() - GAP).max(100.0) } else { 104.0 };
        let rect = Rect::from_min_size(Pos2::new(body.left(), anc.bottom() + GAP), Vec2::new(lw, h));
        let (clicked, _) = glass::tile_at(ui, ("eqtile", 0), rect, eqr.icon, &eqr.title, &eqr.sub, Trailing::Chevron, true);
        if clicked { app.page = Page::Eq; }
        bottom = rect.bottom();
    }
    if !tiles.is_empty() {
        let n = tiles.len();
        let cols = 2usize;
        let rows_n = (n + cols - 1) / cols;
        let avail = if wide { body.bottom() - ty } else { 160.0 * rows_n as f32 + GAP * (rows_n as f32 - 1.0) };
        let th = ((avail - GAP * (rows_n as f32 - 1.0)) / rows_n as f32).clamp(104.0, 170.0);
        let w = ((tw - GAP) / 2.0).floor();
        if !wide { ty = bottom + GAP; }
        for (i, r) in tiles.iter().enumerate() {
            let (cx, cy) = ((i % cols) as f32, (i / cols) as f32);
            let last_odd = i == n - 1 && n % 2 == 1;
            let rect = Rect::from_min_size(Pos2::new(tx0 + cx * (w + GAP), ty + cy * (th + GAP)), Vec2::new(if last_odd { tw } else { w }, th));
            let tr2 = match &r.trailing { Trailing::Switch(v) => Trailing::Switch(*v), Trailing::Chevron => Trailing::Chevron, Trailing::Value(v) => Trailing::Value(v.clone()), Trailing::None => Trailing::None };
            let (clicked, toggled) = glass::tile_at(ui, ("tile", r.id), rect, r.icon, &r.title, &r.sub, tr2, r.enabled);
            if clicked { if let Some(p) = r.page { app.page = p; } }
            if toggled {
                let on_now = matches!(r.trailing, Trailing::Switch(true));
                let cmds = match r.id {
                    "game" => vec![Cmd::LowLatency(!on_now)],
                    "hires" => home_logic::feature_cmds(&app.snap, FEATURE_HIRES, !on_now),
                    "spatial" => home_logic::feature_cmds(&app.snap, FEATURE_SPATIAL, !on_now),
                    _ => vec![],
                };
                // Hi-Res asks both ways, 3D audio only when it turns Hi-Res off (the phone's rules).
                let ask = r.confirm.filter(|_| r.id == "hires" || !on_now);
                match ask {
                    Some(msg) => app.confirm = Some(crate::Confirm { cmds, title: "codec_dialog_title", msg, accept: "codec_dialog_accept" }),
                    None => for c in cmds { app.send(c); },
                }
            }
            bottom = bottom.max(rect.bottom());
        }
    }
    bottom
}

fn battery(app: &mut App, ui: &mut Ui, rect: Rect) {
    glass::flat_at(ui, rect, 28.0, 1.0);
    let s = &app.snap;
    let on = s.status == Status::On;
    let blind = !wear_sensor_seen(s, app.demo);
    let tr = &app.tr;
    // §8: 3/7 in ear, 4 in case, 1/5 out; "out" may only mean the model has no sensor.
    let wear = |i: usize| match s.wear[i] { 3 | 7 => t(tr, "status_in_ear"), 4 => t(tr, "status_in_case"), 1 | 5 => if blind { t(tr, "status_in_ear") } else { t(tr, "status_out") }, _ => "" };
    // Left, case, right.
    let cols = [(0usize, icons::BUD_LEFT), (2, icons::CASE), (1, icons::BUD_RIGHT)];
    // A tall card draws big rings; a short one the compact set. The block sits in the middle of the card.
    let big = rect.height() >= 250.0;
    let (block, ring_r, ring_y, pct_y, sub_y, pct_size) = if big { (210.0, 52.0, 62.0, 150.0, 180.0, 26.0) } else { (160.0, 40.0, 46.0, 112.0, 138.0, 21.0) };
    let oy = ((rect.height() - block) / 2.0).max(0.0);
    let rect = Rect::from_min_max(rect.min + Vec2::new(0.0, oy), rect.max);
    for (n, (i, glyph)) in cols.into_iter().enumerate() {
        let cx = rect.left() + rect.width() * (1.0 / 6.0 + n as f32 / 3.0);
        let b = s.battery[i];
        let frac = b.map(|x| x.0 as f32 / 100.0);
        glass::ring_at(ui, ("ring", n), Pos2::new(cx, rect.top() + ring_y), ring_r, if on { frac } else { None }, Some(glyph), !on);
        let pct = match (b, on) { (Some(x), true) => format!("{}%", x.0), _ => "—".into() };
        glass::text(ui, Pos2::new(cx, rect.top() + pct_y), Align2::CENTER_CENTER, &pct, pct_size, if on { 1.0 } else { 0.5 });
        let sub = if i == 2 { t(tr, "status_case") } else { wear(i) };
        glass::text(ui, Pos2::new(cx, rect.top() + sub_y), Align2::CENTER_CENTER, sub, 13.0, 0.72);
    }
}

fn noise(app: &mut App, ui: &mut Ui, rect: Rect) {
    // A flat plate like the tiles; only the lit pill is glass, and it glides to the chosen mode.
    glass::flat_at(ui, rect, 28.0, 1.0);
    let on = app.snap.status == Status::On;
    let anc = app.snap.anc.clone().unwrap_or_default();
    let is_level = types::LEVELS.contains(&anc.as_str());
    let modes = app.modes.clone();
    let levels = modes.levels();

    // (id, icon, label, selected)
    let mut items: Vec<(String, &'static str, String, bool)> = vec![];
    if app.levels_open && on {
        for id in levels.iter() {
            let (icon, li) = level_info(id);
            items.push((id.to_string(), icon, t(&app.tr, LEVEL_NAMES[li]).into(), anc == *id));
        }
    } else {
        if modes.supports(types::OFF) { items.push((types::OFF.into(), icons::MODE_OFF, t(&app.tr, "anc_seg_off").into(), anc == types::OFF)); }
        if !levels.is_empty() {
            let (icon, li) = level_info(if is_level { &anc } else { "" });
            let mut label = t(&app.tr, "anc_seg_anc").to_string();
            if is_level {
                if let Some(c) = t(&app.tr, LEVEL_NAMES[li]).chars().next() { label = format!("{label} {}", c.to_uppercase()); }
            }
            items.push(("ANC".into(), icon, label, is_level));
        }
        if modes.supports(types::ADAPTIVE) { items.push((types::ADAPTIVE.into(), icons::MODE_ADAPTIVE, t(&app.tr, "anc_seg_adapt").into(), anc == types::ADAPTIVE)); }
        if modes.supports(types::TRANSPARENCY) { items.push((types::TRANSPARENCY.into(), icons::MODE_TRANSPARENCY, t(&app.tr, "anc_seg_trans").into(), anc == types::TRANSPARENCY)); }
    }
    let n = items.len().max(1);
    let pad = 8.0;
    let inner = Rect::from_min_max(rect.min + Vec2::splat(pad), rect.max - Vec2::splat(pad));
    let seg_w = inner.width() / n as f32;
    let sel = if on { items.iter().position(|i| i.3) } else { None };
    // The lit pill, sliding to the selection and fading in and out.
    let target = sel.map_or(0.0, |i| inner.left() + seg_w * i as f32);
    let x = ui.ctx().animate_value_with_time(egui::Id::NULL.with("anc-pill-x"), target, 0.24);
    let vis = ui.ctx().animate_bool_with_time(egui::Id::NULL.with("anc-pill-v"), sel.is_some(), 0.18);
    if vis > 0.0 {
        let r = Rect::from_min_size(Pos2::new(x, inner.top()), Vec2::new(seg_w, inner.height()));
        let rad = 21.0;
        ui.painter().rect_filled(r.expand(3.0), rad + 3.0, white(0.10 * vis));
        ui.painter().rect_filled(r, rad, white(0.36 * vis));
        ui.painter().rect_stroke(r, rad, egui::Stroke::new(1.2, white(0.75 * vis)), egui::StrokeKind::Inside);
    }
    let mut clicked_id = None;
    for (i, (id, icon, label, selected)) in items.iter().enumerate() {
        let seg = Rect::from_min_size(Pos2::new(inner.left() + seg_w * i as f32, inner.top()), Vec2::new(seg_w, inner.height()));
        let r = ui.interact(seg, egui::Id::NULL.with(("anc-seg", i)), egui::Sense::click());
        if r.hovered() && on && !(*selected) { ui.painter().rect_filled(seg.shrink(2.0), 19.0, white(0.07)); }
        let a = if !on { 0.5 } else if *selected { 1.0 } else { 0.82 };
        glass::icon_at(ui, icon, Rect::from_center_size(seg.center() - Vec2::new(0.0, 12.0), Vec2::splat(26.0)), white(a));
        glass::text(ui, seg.center() + Vec2::new(0.0, 22.0), Align2::CENTER_CENTER, label, 12.5, if *selected && on { 1.0 } else { 0.8 * a });
        if on && r.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
        if r.clicked() && on { clicked_id = Some((i, id.clone(), *selected)); }
    }
    if let Some((_, id, selected)) = clicked_id {
        if app.levels_open {
            app.send(Cmd::Anc(if selected { types::OFF.into() } else { id }));
            app.levels_open = false;
        } else if id == "ANC" {
            if levels.len() > 1 { app.levels_open = true; }
            else if let Some(l) = levels.first() { app.send(Cmd::Anc(l.to_string())); }
        } else {
            app.send(Cmd::Anc(id));
        }
    }
}

/// The accept dialog before a write that makes the buds reconnect (`ConfirmDialog` on the phone).
pub fn confirm_dialog(app: &mut App, ui: &mut Ui) {
    let Some(c) = app.confirm.take() else { return };
    let mut decision = None;
    let tr = &app.tr;
    let open = glass::dialog(ui.ctx(), "confirm", t(tr, c.title), Vec2::new(440.0, 230.0), |ui| {
        ui.add(egui::Label::new(egui::RichText::new(t(tr, c.msg)).size(14.0).color(white(0.85))).wrap());
        decision = glass::dialog_buttons(ui, t(tr, "dialog_cancel"), t(tr, c.accept));
    });
    match decision {
        Some(true) => for cmd in c.cmds { app.send(cmd); },
        Some(false) => {}
        None if open => app.confirm = Some(c),
        None => {}
    }
}
