//! QuickBuds for the desktop, egui port. The link thread, the protocol and the model data are the ones of `desktop`
//! (included from there); this crate holds the Glass UI.
#![cfg_attr(windows, windows_subsystem = "windows")]
#![allow(dead_code)]

#[path = "../../desktop/src/bt.rs"]
mod bt;
#[path = "../../desktop/src/protocol.rs"]
mod protocol;
#[path = "../../desktop/src/session.rs"]
mod session;
#[path = "../../desktop/src/types.rs"]
mod types;

mod glass;
mod overview;

pub use buds_core::{cmd, msg, wire};

pub mod icons { include!(concat!(env!("OUT_DIR"), "/icons.rs")); }
pub mod strings { include!(concat!(env!("OUT_DIR"), "/strings.rs")); }

use eframe::egui;
use session::{Cmd, Snapshot, Status};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use types::AncModes;

// ---- helpers `session` expects at the crate root (the same as in `desktop`) ----

#[cfg(not(windows))]
fn local_now() -> [u32; 7] {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    // SAFETY: localtime_r fills the zeroed tm from a valid time_t.
    let tm = unsafe {
        let t = now.as_secs() as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        tm
    };
    [tm.tm_year as u32 + 1900, tm.tm_mon as u32 + 1, tm.tm_mday as u32, tm.tm_hour as u32, tm.tm_min as u32, tm.tm_sec as u32, now.subsec_millis()]
}

#[cfg(windows)]
fn local_now() -> [u32; 7] {
    // SAFETY: GetLocalTime fills the zeroed SYSTEMTIME.
    let st = unsafe {
        let mut st = std::mem::zeroed();
        windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut st);
        st
    };
    [st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond, st.wMilliseconds].map(u32::from)
}

/// `settings.json` in the user's config folder, shared with the Slint app.
fn settings_path() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    let dir = std::env::var_os("APPDATA").map(|d| std::path::PathBuf::from(d).join("QuickBuds"));
    #[cfg(not(windows))]
    let dir = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .map(|d| d.join("quickbuds"));
    dir.map(|d| d.join("settings.json"))
}

fn load_settings() -> serde_json::Value {
    settings_path().and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| serde_json::json!({}))
}

fn save_setting(key: &str, value: serde_json::Value) {
    let Some(path) = settings_path() else { return };
    let mut all = load_settings();
    all[key] = value;
    if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
    let _ = std::fs::write(path, serde_json::to_string_pretty(&all).unwrap_or_default());
}

// ---- strings ----

fn locale_for(tag: &str) -> &'static str {
    let tag = tag.replace('_', "-");
    let parts: Vec<&str> = tag.split(['-', '.']).collect();
    let lang = match parts[0].to_lowercase().as_str() {
        "zh" if parts.iter().any(|p| ["Hant", "TW", "HK", "MO"].contains(p)) => "zh-TW".to_string(),
        "zh" => "zh-CN".into(),
        "id" => "in".into(),
        "tl" => "fil".into(),
        l => l.into(),
    };
    strings::LOCALES.iter().find(|l| l.0 == lang).map_or("", |l| l.0)
}

/// The saved language (`settings.json` "language"), else the system's.
fn chosen_locale() -> &'static str {
    let saved = load_settings()["language"].as_str().unwrap_or("").to_string();
    locale_for(&if saved.is_empty() { sys_locale::get_locale().unwrap_or_default() } else { saved })
}

/// The strings of one locale; a key it lacks falls back to English.
fn translations(locale: &str) -> Vec<String> {
    let table = |tag: &str| strings::LOCALES.iter().find(|l| l.0 == tag).map(|l| l.1);
    let base = table("").unwrap();
    let chosen = table(locale).unwrap_or(base);
    chosen.iter().zip(base).map(|(c, b)| c.or(*b).unwrap_or("").to_string()).collect()
}

pub fn t<'a>(tr: &'a [String], key: &str) -> &'a str {
    &tr[strings::KEYS.iter().position(|k| *k == key).expect(key)]
}

// ---- app ----

#[derive(Clone, Copy, PartialEq)]
pub enum Page { Overview, Eq, Gestures, Hearing, Dual, Earbuds, Settings, Dev }

/// What the connected buds offer; decides which sidebar entries are ready.
#[derive(Clone, Copy, PartialEq)]
pub struct Has { eq: bool, controls: bool, hearing: bool, dual: bool }

impl Has {
    const ALL: Has = Has { eq: true, controls: true, hearing: true, dual: true };
    fn of(s: &Snapshot) -> Has {
        Has {
            eq: types::eq_has(s.model, &s.caps),
            controls: s.caps.supports(types::CMD_SET_KEY_FUNCTION),
            hearing: types::has_feature(&s.features, types::FEATURE_HEARING),
            dual: types::has_feature(&s.features, types::FEATURE_DUAL),
        }
    }
}

pub struct App {
    pub snap: Snapshot,
    pub tx: Sender<Cmd>,
    pub tr: Vec<String>,
    inbox: Arc<Mutex<Option<Snapshot>>>,
    pub page: Page,
    pub has: Has,
    /// Modes of the last buds seen: the controls stay in place (dimmed) while disconnected.
    pub modes: AncModes,
    pub last_level: Option<String>,
    /// The noise control shows the ANC levels instead of the modes.
    pub levels_open: bool,
    pub glow: bool,
    pub demo: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        glass::install(cc);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let inbox = Arc::new(Mutex::new(None));
        let demo = std::env::var_os("QB_DEMO").is_some();
        let tx = if demo {
            // A fixed snapshot and no link: for screenshots and UI tests, so no buds are touched.
            *inbox.lock().unwrap() = Some(overview::demo_snapshot());
            std::sync::mpsc::channel().0
        } else {
            let (inbox, ctx) = (inbox.clone(), cc.egui_ctx.clone());
            session::spawn(move |s| { *inbox.lock().unwrap() = Some(s); ctx.request_repaint(); })
        };
        let glow = load_settings()["glass_gradient"].as_bool().unwrap_or(true);
        Self {
            snap: Snapshot::default(), tx, tr: translations(chosen_locale()), inbox, page: Page::Overview, has: Has::ALL,
            modes: AncModes::default(), last_level: None, levels_open: false, glow, demo,
        }
    }

    fn apply(&mut self, s: Snapshot) {
        if s.status == Status::On {
            self.modes = s.modes.clone();
            if Has::of(&s) != self.has { self.has = Has::of(&s); }
        }
        if let Some(a) = s.anc.as_deref().filter(|a| types::LEVELS.contains(a)) { self.last_level = Some(a.into()); }
        self.snap = s;
    }

    pub fn send(&self, c: Cmd) { let _ = self.tx.send(c); }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let new = self.inbox.lock().unwrap().take();
        if let Some(s) = new { self.apply(s); }
        glass::set_glow(ui.ctx(), if self.glow { 1.0 } else { 0.0 });
        glass::backdrop(ui);
        overview::show(self, ui);
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([760.0, 560.0])
            .with_title("QuickBuds")
            .with_app_id("quickbuds"),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native("QuickBuds", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
