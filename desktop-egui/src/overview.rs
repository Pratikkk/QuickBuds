//! The window: sidebar, header and the Overview page, on the glass widgets.

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

/// The undecorated window's edges: the cursor and the system resize on a press near an edge.
fn resize_edges(ui: &Ui) {
    use egui::{CursorIcon as C, ResizeDirection as D};
    let ctx = ui.ctx();
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)) { return; }
    let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else { return };
    let r = ctx.content_rect();
    let e = 6.0;
    let (l, rt, t, b) = (p.x - r.left() < e, r.right() - p.x < e, p.y - r.top() < e, r.bottom() - p.y < e);
    let (dir, cursor) = match (l, rt, t, b) {
        (true, _, true, _) => (D::NorthWest, C::ResizeNwSe),
        (_, true, true, _) => (D::NorthEast, C::ResizeNeSw),
        (true, _, _, true) => (D::SouthWest, C::ResizeNeSw),
        (_, true, _, true) => (D::SouthEast, C::ResizeNwSe),
        (true, ..) => (D::West, C::ResizeHorizontal),
        (_, true, ..) => (D::East, C::ResizeHorizontal),
        (_, _, true, _) => (D::North, C::ResizeVertical),
        (_, _, _, true) => (D::South, C::ResizeVertical),
        _ => return,
    };
    ctx.set_cursor_icon(cursor);
    if ctx.input(|i| i.pointer.primary_pressed()) { ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir)); }
}

/// A strip that moves the window when dragged and maximizes it on a double click.
fn drag_strip(ui: &mut Ui, id: &str, rect: Rect) {
    let r = ui.interact(rect, ui.scope_id().with(id), egui::Sense::click_and_drag());
    if r.drag_started_by(egui::PointerButton::Primary) { ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag); }
    if r.double_clicked() {
        let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    resize_edges(ui);
    let full = ui.max_rect();
    let sb = Rect::from_min_size(full.min + Vec2::splat(12.0), Vec2::new(236.0, full.height() - 24.0));
    sidebar(app, ui, sb);
    let content = Rect::from_min_max(Pos2::new(sb.right() + GAP, full.top() + 18.0), Pos2::new(full.right() - 12.0, full.bottom() - 12.0));
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

fn sidebar(app: &mut App, ui: &mut Ui, sb: Rect) {
    glass::glass_at(ui, sb, 32.0, 1.0);
    drag_strip(ui, "drag-sidebar", Rect::from_min_size(sb.min, Vec2::new(sb.width(), 70.0)));
    glass::icon_at(ui, icons::BUD_LEFT, Rect::from_center_size(Pos2::new(sb.left() + 34.0, sb.top() + 38.0), Vec2::new(18.0, 24.0)), white(1.0));
    glass::text(ui, Pos2::new(sb.left() + 52.0, sb.top() + 38.0), Align2::LEFT_CENTER, "QuickBuds", 17.0, 1.0);
    let h = app.has;
    let top = [
        (Page::Overview, icons::LAYOUT, "desktop_nav_overview", true),
        (Page::Eq, icons::EQUALIZER, "eq_title", h.eq),
        (Page::Gestures, icons::GESTURE, "gesture_title", h.controls),
        (Page::Hearing, icons::HEARING, "row_golden_title", h.hearing),
        (Page::Dual, icons::DEVICES, "dual_title", h.dual),
        (Page::Earbuds, icons::EARBUD, "earbuds_title", true),
    ];
    let bottom = [(Page::Settings, icons::SETTINGS_COG, "desktop_nav_app_settings", true), (Page::Dev, icons::DEV_TOOLS, "action_dev_tools", true)];
    let bottom_y = sb.bottom() - 12.0 - 44.0 * bottom.len() as f32 - 2.0;
    let mut items: Vec<(Page, &'static str, &'static str, bool, Rect)> = vec![];
    for (i, (page, icon, key, ready)) in top.into_iter().enumerate() {
        items.push((page, icon, key, ready, Rect::from_min_size(Pos2::new(sb.left() + 12.0, sb.top() + 76.0 + 46.0 * i as f32), Vec2::new(sb.width() - 24.0, 44.0))));
    }
    for (i, (page, icon, key, ready)) in bottom.into_iter().enumerate() {
        items.push((page, icon, key, ready, Rect::from_min_size(Pos2::new(sb.left() + 12.0, bottom_y + 46.0 * i as f32), Vec2::new(sb.width() - 24.0, 44.0))));
    }
    let on_page = |p: Page| p == app.page || (p == Page::Earbuds && app.page == Page::Models);
    // The lit pill slides to the selected entry.
    if let Some(sel) = items.iter().find(|i| on_page(i.0)).map(|i| i.4) {
        let ty = ui.ctx().animate_value_with_time(egui::Id::NULL.with("nav-pill"), sel.top(), 0.22);
        ui.painter().rect_filled(Rect::from_min_size(Pos2::new(sel.left(), ty), sel.size()), 22.0, glass::sel_fill(1.0));
    }
    let mut go = None;
    for (page, icon, key, ready, r) in &items {
        if glass::nav_item_at(ui, *key, *r, icon, t(&app.tr, key), on_page(*page), *ready).clicked() && *ready { go = Some(*page); }
    }
    if let Some(p) = go { app.page = p; }
}

fn header(app: &mut App, ui: &mut Ui, c: Rect) {
    let s = &app.snap;
    let title = if s.name.is_empty() { "QuickBuds" } else { s.name.as_str() };
    // The window has no title bar: the header and the top of the sidebar move it.
    drag_strip(ui, "drag-header", Rect::from_min_max(Pos2::new(c.left(), c.top() - 18.0), Pos2::new(c.right(), c.top() + 52.0)));
    glass::text(ui, Pos2::new(c.left() + 4.0, c.top() + 23.0), Align2::LEFT_CENTER, title, 30.0, 1.0);
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

    // Rows on the right: the switches the buds list. Hi-Res and 3D audio show their state; their writes come with the Earbud settings port.
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
    if app.has.eq { left.push(nav("eq", icons::EQUALIZER, "eq_title", "", Page::Eq)); }
    if app.has.dual { left.push(nav("dual", icons::DEVICES, "dual_title", "", Page::Dual)); }
    left.push(nav("earbuds", icons::EARBUD, "earbuds_title", "", Page::Earbuds));

    // Layout: two columns when wide, one when narrow.
    let wide = body.width() >= 820.0;
    let (c1, c2) = if wide {
        let w1 = ((body.width() - GAP) * 0.535).floor();
        (Rect::from_min_size(body.min, Vec2::new(w1, body.height())), Rect::from_min_max(Pos2::new(body.left() + w1 + GAP, body.top()), body.max))
    } else {
        (body, body)
    };
    let h_batt = 224.0;
    let h_anc = 114.0;
    let h_right = ROW * right.len() as f32;
    let h_left = ROW * left.len() as f32;
    let batt = Rect::from_min_size(c1.min, Vec2::new(c1.width(), h_batt));
    let (anc, rows_r, nav_r);
    if wide {
        anc = Rect::from_min_size(c2.min, Vec2::new(c2.width(), h_anc));
        rows_r = Rect::from_min_size(Pos2::new(c2.left(), anc.bottom() + GAP), Vec2::new(c2.width(), h_right));
        nav_r = Rect::from_min_size(Pos2::new(c1.left(), batt.bottom() + GAP), Vec2::new(c1.width(), h_left));
    } else {
        anc = Rect::from_min_size(Pos2::new(c1.left(), batt.bottom() + GAP), Vec2::new(c1.width(), h_anc));
        rows_r = Rect::from_min_size(Pos2::new(c1.left(), anc.bottom() + GAP), Vec2::new(c1.width(), h_right));
        nav_r = Rect::from_min_size(Pos2::new(c1.left(), rows_r.bottom() + GAP), Vec2::new(c1.width(), h_left));
    }

    let bottom = [batt.bottom(), anc.bottom(), rows_r.bottom(), nav_r.bottom()].into_iter().fold(f32::MIN, f32::max);
    battery(app, ui, batt);
    noise(app, ui, anc);
    for (rect, list, id) in [(rows_r, right, "right"), (nav_r, left, "left")] {
        if list.is_empty() { continue; }
        glass::flat_at(ui, rect, 32.0, 1.0);
        for (i, r) in list.iter().enumerate() {
            let rr = Rect::from_min_size(Pos2::new(rect.left(), rect.top() + ROW * i as f32), Vec2::new(rect.width(), ROW));
            if i > 0 { glass::divider_at(ui, rect.left(), rect.right(), rr.top()); }
            let tr = match &r.trailing { Trailing::Switch(v) => Trailing::Switch(*v), Trailing::Chevron => Trailing::Chevron, Trailing::Value(v) => Trailing::Value(v.clone()), Trailing::None => Trailing::None };
            let (clicked, toggled) = glass::row_at(ui, (id, r.id), rr, r.icon, &r.title, &r.sub, tr, r.enabled);
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
        }
    }
    bottom
}

fn battery(app: &mut App, ui: &mut Ui, rect: Rect) {
    glass::flat_at(ui, rect, 32.0, 1.0);
    let s = &app.snap;
    let on = s.status == Status::On;
    let blind = !wear_sensor_seen(s, app.demo);
    let tr = &app.tr;
    // §8: 3/7 in ear, 4 in case, 1/5 out; "out" may only mean the model has no sensor.
    let wear = |i: usize| match s.wear[i] { 3 | 7 => t(tr, "status_in_ear"), 4 => t(tr, "status_in_case"), 1 | 5 => if blind { t(tr, "status_in_ear") } else { t(tr, "status_out") }, _ => "" };
    // Left, case, right.
    let cols = [(0usize, icons::BUD_LEFT), (2, icons::CASE), (1, icons::BUD_RIGHT)];
    for (n, (i, glyph)) in cols.into_iter().enumerate() {
        let cx = rect.left() + rect.width() * (1.0 / 6.0 + n as f32 / 3.0);
        let b = s.battery[i];
        let frac = b.map(|x| x.0 as f32 / 100.0);
        glass::ring_at(ui, ("ring", n), Pos2::new(cx, rect.top() + 66.0), 40.0, if on { frac } else { None }, Some(glyph), !on);
        let pct = match (b, on) { (Some(x), true) => format!("{}%", x.0), _ => "—".into() };
        glass::text(ui, Pos2::new(cx, rect.top() + 134.0), Align2::CENTER_CENTER, &pct, 21.0, if on { 1.0 } else { 0.5 });
        let sub = if i == 2 { t(tr, "status_case") } else { wear(i) };
        glass::text(ui, Pos2::new(cx, rect.top() + 160.0), Align2::CENTER_CENTER, sub, 13.0, 0.72);
    }
    if !s.name.is_empty() {
        glass::text(ui, Pos2::new(rect.center().x, rect.top() + 196.0), Align2::CENTER_CENTER, &s.name, 14.0, 0.72);
    }
}

fn noise(app: &mut App, ui: &mut Ui, rect: Rect) {
    glass::glass_at(ui, rect, 32.0, 1.0);
    let on = app.snap.status == Status::On;
    let anc = app.snap.anc.clone().unwrap_or_default();
    let is_level = types::LEVELS.contains(&anc.as_str());
    let modes = app.modes.clone();
    let levels = modes.levels();
    let cy = rect.top() + 46.0;

    if app.levels_open && on {
        let n = levels.len().max(1);
        for (i, id) in levels.iter().enumerate() {
            let (icon, li) = level_info(id);
            let cx = rect.left() + rect.width() * (i as f32 + 0.5) / n as f32;
            let r = glass::disc_at(ui, ("level", i), Pos2::new(cx, cy), 54.0, icon, t(&app.tr, LEVEL_NAMES[li]), anc == *id);
            if r.clicked() {
                app.send(Cmd::Anc(if anc == *id { types::OFF.into() } else { id.to_string() }));
                app.levels_open = false;
            }
        }
        return;
    }

    // (mode id, icon, label)
    let mut items: Vec<(String, &'static str, String)> = vec![];
    if modes.supports(types::OFF) { items.push((types::OFF.into(), icons::MODE_OFF, t(&app.tr, "anc_seg_off").into())); }
    if !levels.is_empty() {
        let (icon, li) = level_info(if is_level { &anc } else { "" });
        let mut label = t(&app.tr, "anc_seg_anc").to_string();
        if is_level {
            if let Some(c) = t(&app.tr, LEVEL_NAMES[li]).chars().next() { label = format!("{label} {}", c.to_uppercase()); }
        }
        items.push(("ANC".into(), icon, label));
    }
    if modes.supports(types::ADAPTIVE) { items.push((types::ADAPTIVE.into(), icons::MODE_ADAPTIVE, t(&app.tr, "anc_seg_adapt").into())); }
    if modes.supports(types::TRANSPARENCY) { items.push((types::TRANSPARENCY.into(), icons::MODE_TRANSPARENCY, t(&app.tr, "anc_seg_trans").into())); }
    let n = items.len().max(1);
    for (i, (id, icon, label)) in items.iter().enumerate() {
        let cx = rect.left() + rect.width() * (i as f32 + 0.5) / n as f32;
        let selected = if id == "ANC" { is_level } else { anc == *id };
        let r = glass::disc_at(ui, ("mode", i), Pos2::new(cx, cy), 54.0, icon, label, selected && on);
        if r.clicked() && on {
            if id == "ANC" {
                if levels.len() > 1 { app.levels_open = true; }
                else if let Some(l) = levels.first() { app.send(Cmd::Anc(l.to_string())); }
            } else {
                app.send(Cmd::Anc(id.clone()));
            }
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
