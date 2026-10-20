//! The link to the buds on its own thread: finds them, runs the init sequence, applies pushes and
//! sends the UI's commands. Mirrors `BudsConnectionManager`: nothing is polled after connect.

use crate::bt;
use crate::cmd;
use crate::types::*;
use crate::proto::Proto;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub enum Cmd {
    Anc(String),
    LowLatency(bool),
    EqBuiltIn(u8),
    /// Select and save a custom preset (a band edit or rename is the same frame).
    EqSave(Preset),
    EqCreate(Preset),
    EqDelete(Preset),
    BassWave(bool),
    BassLevel(i8),
    /// `0x0403` switches in this order, then the status re-read.
    Features(Vec<(u8, bool)>),
    /// The locator tone on both buds.
    Find(bool),
    /// The model list's pick changed (`settings.json` "model_manual"): the model is looked up again.
    ModelPicked,
    /// Earbud settings opened: the reads the phone's screen does then (alert volume, tap sensitivity).
    EarbudReads,
    AlertVolume(u8),
    TapLevel(u8),
    GameSoundType(u8),
    /// `0x0422`: 0 off, 1 fixed, 2 head tracking, then the read-back.
    SpatialType(u8),
    /// `0x041A` codec and Hi-Res; the buds restart and the link comes back by itself.
    Codec(u8, bool),
    HeadMotion(u8),
    /// `0x0405` start / stop.
    FitTest(bool),
    /// Personalized ANC: `0x011A` (is a result stored?).
    PncQuery,
    /// `0x0412 <action>`; using the stored result re-reads the status after it.
    Pnc(u8),
    /// `0x010D`, after a Personalized ANC result applied (the buds switch `0C` on themselves).
    StatusRead,
    /// The Dual connection switch.
    Dual(bool),
    /// Dual connection opened: the device list, the status, and the preferred device if shown.
    DualReads(bool),
    /// Connect (true) or disconnect another device on the list.
    ConnectDevice([u8; 6], bool),
    /// None = automatic.
    Preferred(Option<[u8; 6]>),
    /// Controls opened: the gesture table and the hold cycles of these noise types.
    ControlsReads(Vec<u8>),
    /// One gesture on one bud: (dev, act, fn), written into every slot that bud has for it, outside on-call.
    Gesture(u8, u8, u8),
    /// The hold's noise cycle: (mask, type).
    HoldModes(u32, u8),
    /// An on-call row: (act, fn), one entry for both buds.
    /// A both-buds entry (`dev 04`): button, act, fn.
    OnCall(u8, u8, u8),
    /// Hearing profile frames, in order; a switch among them re-reads the status after.
    Hearing(Vec<Frame>),
    Connect,
    Disconnect,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Status {
    #[default]
    Off,
    Connecting,
    On,
}

#[derive(Clone, Default)]
pub struct Snapshot {
    pub status: Status,
    pub name: String,
    /// Left, right, case: (level, charging).
    pub battery: [Option<(u8, bool)>; 3],
    /// Left, right, case: raw wear status (§8), 0 = unknown.
    pub wear: [u8; 3],
    /// The events the buds offer (`0x8200`), once read.
    pub offered: Option<Vec<u8>>,
    pub anc: Option<String>,
    pub modes: AncModes,
    pub low_latency: Option<bool>,
    /// The model in use: the manual pick, else [detected](Self::detected).
    pub model: Option<&'static serde_json::Value>,
    /// What the product id (else the name) says.
    pub detected: Option<&'static serde_json::Value>,
    /// The model was picked by hand: its `models.json` flags count as features (`Capabilities.offered`).
    pub manual: bool,
    pub caps: Caps,
    pub firmware: Option<String>,
    pub eq_current: Option<u8>,
    pub eq_custom: Vec<Preset>,
    pub bass_on: Option<bool>,
    pub bass_level: Option<i8>,
    /// `0x810D`: every feature id the buds listed, with its value.
    pub features: Vec<(u8, u8)>,
    pub alert_volume: Option<u8>,
    /// (level, the buds' default)
    pub tap_level: Option<(u8, u8)>,
    /// (selected, offered types) from `0x812B`.
    pub game_sound: Option<(u8, Vec<u8>)>,
    pub head_motion: Option<u8>,
    /// The level Smart chose (`ANC-Light` ...), §5.
    pub smart_level: Option<String>,
    /// Counts ANC writes refused with no bud in an ear (status 14, §5): each one shows the hint.
    pub anc_refused: u32,
    /// `0x812A` / push `0x0510`: 0 off, 1 fixed, 2 head tracking.
    pub spatial_type: Option<u8>,
    /// The current codec (`0x8114`) and the offered ones (`0x8123`), §9 Codec picker.
    pub codec: Option<u8>,
    pub codecs: Vec<u8>,
    /// The last fit test result, and a count that changes with each one.
    pub fit: (u8, u8, u32),
    /// The last Personalized ANC event (1 stored, 2 ack, 3 result), its value, and a count that changes with each.
    pub pnc: (u8, u8, u32),
    /// The buds' paired-device list (Dual connection).
    pub devices: Vec<PairedDevice>,
    /// None = automatic.
    pub preferred: Option<[u8; 6]>,
    /// The last gesture table read, `[dev, btn, act, fn]`; None until read (no write without it).
    pub keyfn: Option<Vec<[u8; 4]>>,
    /// The hold's noise cycles read: (type, mask).
    pub hold_masks: Vec<(u8, u32)>,
}

type Emit = Box<dyn Fn(Snapshot) + Send>;

/// One packet log line (Dev tools).
pub struct Line {
    /// Local time, "18:23:05.123".
    pub at: String,
    /// "TX", "RX", "DISCARDED RX", or "NOTE" (a connection change in words, no bytes).
    pub dir: &'static str,
    pub bytes: Vec<u8>,
    /// The decoded description; None = a packet nothing here names (amber in Human).
    pub human: Option<String>,
}

// ponytail: unbounded; a cap if a days-long session ever shows in memory.
pub static LOG: Mutex<Vec<Line>> = Mutex::new(Vec::new());

/// Hearing profile replies and pushes, in order, for the page and the test to take (`hearing::apply`).
pub static HEARING: Mutex<Vec<HearingEv>> = Mutex::new(Vec::new());

pub(crate) fn log(dir: &'static str, bytes: Vec<u8>, human: Option<String>) {
    let [.., h, mi, sec, ms] = crate::local_now();
    LOG.lock().unwrap().push(Line { at: format!("{h:02}:{mi:02}:{sec:02}.{ms:03}"), dir, bytes, human });
}

pub fn spawn(emit: impl Fn(Snapshot) + Send + 'static) -> Sender<Cmd> {
    let (tx, rx) = channel();
    std::thread::spawn(move || run(rx, Box::new(emit)));
    tx
}

enum End { Lost, UserDisconnect, Quit }

/// The manual pick, else the detected model; the ANC modes follow it.
fn pick_model(s: &mut Snapshot) {
    let manual = manual_model();
    s.manual = manual.is_some();
    s.model = manual.or(s.detected);
    s.modes = AncModes::of(s.model);
}

/// The model picked by hand in the model list, if any.
fn manual_model() -> Option<&'static serde_json::Value> {
    let id = crate::load_settings()["model_manual"].as_str()?.to_string();
    models().iter().find(|m| m["id"].as_str() == Some(&id))
}

/// Addresses that answered as buds before (`settings.json` "buds"), for systems that do not list the vendor service.
fn remembered() -> Vec<u64> {
    crate::load_settings()["buds"].as_array().into_iter().flatten()
        .filter_map(|v| u64::from_str_radix(v.as_str()?, 16).ok()).collect()
}

fn remember(addr: u64) {
    let mut list = remembered();
    if addr == 0 || list.contains(&addr) { return; }
    list.push(addr);
    crate::save_setting("buds", list.iter().map(|a| format!("{a:012X}")).collect::<Vec<_>>().into());
}

fn run(rx: Receiver<Cmd>, emit: Emit) {
    let mut paused = false;
    let mut manual = false;
    loop {
        if !paused {
            // Auto-connect follows audio: only buds the system has a link to. Buds are found by the vendor's
            // control service (`BudsDevice.find`), or as the device that answered before; a known model name
            // is the last resort. A user Connect also tries any other connected device.
            let known = remembered();
            let rank = |d: &bt::Device| if d.vendor || known.contains(&d.addr) { 0 } else if is_known_name(&d.name) { 1 } else { 2 };
            let mut devices: Vec<_> = bt::paired().into_iter().filter(|d| d.connected && (manual || rank(d) < 2)).collect();
            devices.sort_by_key(rank);
            for d in devices {
                emit(Snapshot { status: Status::Connecting, name: d.name.clone(), ..Default::default() });
                let Ok(link) = bt::connect(d.addr) else { continue };
                log("NOTE", Vec::new(), Some(format!("Connected to {}", d.name)));
                let mut c = Conn::new(link, d.name, d.addr, &emit);
                match c.serve(&rx) {
                    End::Lost => log("NOTE", Vec::new(), Some("Connection lost".into())),
                    End::UserDisconnect => { log("NOTE", Vec::new(), Some("Disconnected".into())); paused = true }
                    End::Quit => return,
                }
                break;
            }
            emit(Snapshot::default());
        }
        manual = false;
        // ponytail: 5 s rescan while idle; a Windows device-change notification if this ever costs.
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Cmd::Connect) => { paused = false; manual = true; }
            Ok(Cmd::Disconnect) => paused = true,
            Ok(_) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

struct Conn<'a> {
    link: bt::Link,
    /// 0 for the bridge.
    addr: u64,
    /// The link's protocol (docs/PROTOCOL.md), chosen once per connection.
    proto: Proto,
    /// Messages received on this connection: none after the init sequence means the new protocol failed.
    rx: u32,
    seq: u8,
    /// The last ANC level sent, to name a report that says only "noise cancelling".
    level: Option<String>,
    s: Snapshot,
    emit: &'a Emit,
}

impl<'a> Conn<'a> {
    fn new(link: bt::Link, name: String, addr: u64, emit: &'a Emit) -> Self {
        let next = crate::protocol_next(addr);
        crate::types::use_legacy_db(!next);
        let model = find_model(None, Some(&name));
        let mut s = Snapshot { status: Status::On, name, detected: model, ..Default::default() };
        pick_model(&mut s);
        log("NOTE", Vec::new(), Some(format!("Protocol: {}{}", if next { "new" } else { "legacy" }, if crate::fell_back(addr) { " (fell back for these buds)" } else { "" })));
        Conn { link, addr, proto: Proto::new(next), rx: 0, seq: 0, level: None, s, emit }
    }

    /// Sends a frame built by `buds_core::cmd`.
    fn tx(&mut self, f: Frame) -> Result<(), String> { self.send(f.0, None, &f.1) }

    fn send(&mut self, cmd: u16, seq: Option<u8>, payload: &[u8]) -> Result<(), String> {
        let seq = seq.unwrap_or_else(|| { self.seq = self.seq % 0xFE + 1; self.seq });
        let p = build_packet(cmd, seq, payload);
        log("TX", p.clone(), describe_tx(cmd, payload));
        // A message too big for one frame goes out in slices (PROTOCOL.md §2).
        for f in self.proto.frames_out(cmd, seq, payload) { self.link.write(&f)?; }
        Ok(())
    }

    /// Reads and applies packets for `ms`.
    fn pump(&mut self, ms: u64) -> Result<(), String> {
        let end = Instant::now() + Duration::from_millis(ms);
        let mut buf = [0u8; 1024];
        loop {
            let n = self.link.read(&mut buf)?;
            let mut changed = false;
            let frames = self.proto.frames_in(&buf[..n]);
            self.rx += frames.len() as u32;
            for p in frames.into_iter().flat_map(|p| {
                // A batch reply is logged as it came, then handled as the replies it carries.
                if cmd_of(&p) != CMD_BATCH_REPLY { return vec![p]; }
                let parts = unbatch(&p);
                log("RX", p, Some(format!("batch reply, {} answers", parts.len())));
                parts
            }) {
                let e = self.proto.decode(&p);
                let human = e.as_ref().map(describe).or_else(|| describe_other(&p));
                // Some buds drop the link a few seconds after an unanswered request (issue #2: the time request).
                let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as u32);
                let answer = self.proto.answer(&p, now);
                // A refused ANC write (status 14 with no bud in an ear): read the real mode back (§5).
                let refused = cmd_of(&p) == CMD_SET_ANC | 0x8000 && payload_of(&p).first().is_some_and(|&st| st != 0);
                let need_ear = refused && payload_of(&p).first() == Some(&0x0E);
                log("RX", p, human);
                if let Some(((cmd, pl), seq)) = answer { self.send(cmd, Some(seq), &pl)?; }
                if refused { self.tx(cmd::anc_read(1, 1))?; }
                if need_ear { self.s.anc_refused += 1; changed = true; }
                if let Some(e) = e { changed |= self.apply(e); }
            }
            let skipped = self.proto.take_discarded();
            if !skipped.is_empty() { log("DISCARDED RX", skipped, None); }
            if changed { (self.emit)(self.s.clone()); }
            if Instant::now() >= end { return Ok(()); }
        }
    }

    fn apply(&mut self, e: Event) -> bool {
        let s = &mut self.s;
        match e {
            Event::Caps(c) => s.caps = c,
            Event::ProductId(id) => {
                // Other buds than the manual pick was made for: back to Automatic (`ModelCatalog`).
                let all = crate::load_settings();
                if all["product_id"].as_str() != Some(&id) {
                    crate::save_setting("product_id", id.as_str().into());
                    if !all["model_manual"].is_null() { crate::save_setting("model_manual", serde_json::Value::Null); }
                }
                s.detected = find_model(Some(&id), Some(&s.name));
                pick_model(s);
                // The bridge (or an unknown device name) shows the model's name instead.
                if let Some(n) = s.model.and_then(|m| m["name"].as_str()) {
                    if !is_known_name(&s.name) { s.name = n.to_string(); }
                }
            }
            Event::Firmware(f) => s.firmware = Some(f),
            Event::EqCurrent(id) => s.eq_current = Some(id),
            Event::Offered(o) => s.offered = Some(o),
            Event::EqCustom(list) => s.eq_custom = list,
            Event::BassLevel(l) => s.bass_level = Some(l),
            Event::Battery(v) => {
                // Settings › Hide old battery levels: a part the report leaves out is cleared, not kept.
                if crate::load_settings()["clear_battery"].as_bool() == Some(true) { s.battery = [None; 3]; }
                for (i, level, charging) in v {
                    if (1..=3).contains(&i) { s.battery[i as usize - 1] = Some((level, charging)); }
                }
            }
            Event::Wear(v) => for (i, st) in v {
                if (1..=3).contains(&i) { s.wear[i as usize - 1] = st; }
            },
            Event::AncRaw(raw) => match s.modes.mode_for_raw(raw, self.level.as_deref()) {
                Some(m) => s.anc = Some(m),
                None => return false,
            },
            Event::GameMode(pl) => match game_on(&pl, crate::load_settings()["product_id"].as_str()) {
                Some(on) => s.low_latency = Some(on),
                None => return false,
            },
            Event::AlertVolume(l) => s.alert_volume = Some(l),
            Event::TapLevel(l, d) => s.tap_level = Some((l, d)),
            Event::TapPush(l) => s.tap_level = Some((l, s.tap_level.map_or(0, |t| t.1))),
            Event::GameSound(t, all) => s.game_sound = Some((t, all)),
            Event::HeadMotion(t) => s.head_motion = Some(t),
            Event::SmartLevel(m) => s.smart_level = (m != 0).then(|| s.modes.level_for_bit(m.trailing_zeros() as u8)).flatten().map(Into::into),
            Event::SpatialType(t) => s.spatial_type = Some(t),
            Event::Codec(c) => s.codec = Some(c),
            Event::Codecs(c) => s.codecs = c,
            Event::FitResult(l, r) => s.fit = (l, r, s.fit.2 + 1),
            Event::PncStored(e) => s.pnc = (1, e as u8, s.pnc.2 + 1),
            Event::PncAck(st) => s.pnc = (2, st, s.pnc.2 + 1),
            Event::PncResult(r) => s.pnc = (3, r, s.pnc.2 + 1),
            Event::Devices(d) => s.devices = d,
            Event::Preferred(m) => s.preferred = m,
            Event::Hearing(h) => HEARING.lock().unwrap().push(h),
            Event::KeyFunctions(t) => s.keyfn = Some(t),
            Event::HoldMask(k, m) => match s.hold_masks.iter_mut().find(|x| x.0 == k) { Some(x) => x.1 = m, None => s.hold_masks.push((k, m)) },
            Event::Features(f) => {
                let get = |id: u8| f.iter().find(|x| x.0 == id).map(|x| x.1 == 1);
                s.low_latency = get(s.caps.game_mode_id()).or(s.low_latency);
                s.bass_on = get(FEATURE_BASSWAVE).or(s.bass_on);
                for (id, v) in f {
                    match s.features.iter_mut().find(|x| x.0 == id) { Some(x) => x.1 = v, None => s.features.push((id, v)) }
                }
            }
        }
        true
    }

    fn serve(&mut self, rx: &Receiver<Cmd>) -> End {
        (self.emit)(self.s.clone());
        let init = self.init();
        // Buds that named their model are remembered, renamed or not.
        if init.is_ok() && self.s.model.is_some() { remember(self.addr); }
        // The new protocol got no answer at all: these buds use the old link from the next connect on (Android does the same).
        if self.proto.is_new() && self.rx == 0 {
            crate::mark_fell_back(self.addr);
            log("NOTE", Vec::new(), Some("PROTOCOL FALLBACK: no answer. Reconnecting on the legacy protocol.".into()));
            return End::Lost;
        }
        match init.and_then(|_| self.run(rx)) {
            Ok(end) => end,
            Err(_) => End::Lost,
        }
    }

    /// PROTOCOL-LEGACY.md "Init sequence": 300 ms before the first frame, then 200 ms apart; reads only if listed.
    fn init(&mut self) -> Result<(), String> {
        self.pump(300)?;
        for f in [cmd::handshake(), cmd::product_id(), cmd::broadcast_codes()] {
            self.tx(f)?;
            self.pump(200)?;
        }
        let ids = notify_ids(&self.s.caps, self.s.offered.as_deref());
        if !self.s.caps.supports(CMD_REGISTER_NOTIFY) {
            for &id in &ids { self.tx(cmd::register_one(id))?; }
            self.pump(200)?;
        }
        let smart = self.s.modes.supports("ANC-Smart");
        let queries: [(u16, Option<u8>, Vec<u8>); 7] = [
            (CMD_REGISTER_NOTIFY, None, cmd::register(&ids).1),
            (CMD_QUERY_STATUS, Some(0x00), status_query(self.s.model)),
            (CMD_QUERY_ANC, None, cmd::anc_read(1, 1).1),
            // The level Smart chose, §5.
            (CMD_QUERY_ANC, None, if smart { cmd::anc_read(4, 1).1 } else { vec![] }),
            (CMD_QUERY_BATTERY, Some(0xF0), vec![]),
            (CMD_QUERY_WEARING, Some(0xF2), vec![]),
            (CMD_QUERY_FIRMWARE, None, vec![]),
        ];
        // Buds with the `0x012F` batch get the reads in one frame, as HeyMelody sends them.
        let batch = self.s.caps.supports(CMD_BATCH);
        let mut reads = Vec::new();
        for (cmd, seq, payload) in queries {
            if !self.s.caps.supports(cmd) || cmd == CMD_REGISTER_NOTIFY && ids.is_empty() || cmd == CMD_QUERY_ANC && payload.is_empty() { continue; }
            if batch && cmd != CMD_REGISTER_NOTIFY { reads.push((cmd, payload)); continue; }
            self.send(cmd, seq, &payload)?;
            self.pump(200)?;
        }
        if batch {
            reads.extend([CMD_QUERY_EQ, CMD_QUERY_EQ_ALL, CMD_QUERY_BASSWAVE].into_iter()
                .filter(|&c| self.s.caps.supports(c)).map(|c| (c, vec![])));
            if !reads.is_empty() { self.tx(cmd::batch(&reads))?; self.pump(200)?; }
        } else {
            self.eq_reads()?;
        }
        // The Overview's 3D audio type and codec picker, as the phone reads them.
        for cmd in [CMD_QUERY_SPATIAL_TYPE, CMD_QUERY_CODEC_LIST, CMD_QUERY_CODEC] {
            if !self.s.caps.supports(cmd) { continue; }
            self.send(cmd, None, &[])?;
            self.pump(120)?;
        }
        // realme Link hands 3D audio to the buds on every connect; else the `1B` switch can do nothing (issue #9).
        if self.s.model.is_some_and(|m| m["spatialToggle"] == 1) && self.s.caps.supports(CMD_SPATIAL_SOURCE) {
            self.tx(cmd::spatial_source(0))?;
            self.pump(120)?;
        }
        Ok(())
    }

    /// The EQ reads these buds list, in order (`sendThenRead`); after a write too, since only the buds know
    /// the ids after a create or delete.
    fn eq_reads(&mut self) -> Result<(), String> {
        for cmd in [CMD_QUERY_EQ, CMD_QUERY_EQ_ALL, CMD_QUERY_BASSWAVE] {
            if !self.s.caps.supports(cmd) { continue; }
            self.send(cmd, None, &[])?;
            self.pump(120)?;
        }
        Ok(())
    }

    /// A write, then the EQ re-read.
    /// A preset selection only: a built-in, or a custom preset saved unchanged.
    fn eq_pick(&self, c: &Cmd) -> bool {
        match c {
            Cmd::EqBuiltIn(_) => true,
            Cmd::EqSave(p) => self.s.eq_custom.contains(p),
            _ => false,
        }
    }

    fn eq_write(&mut self, f: Frame) -> Result<(), String> {
        (self.emit)(self.s.clone());
        self.tx(f)?;
        self.pump(250)?;
        self.eq_reads()
    }

    fn run(&mut self, rx: &Receiver<Cmd>) -> Result<End, String> {
        loop {
            self.pump(0)?;
            let mut cmds = Vec::new();
            loop {
                match rx.try_recv() {
                    Ok(c) => cmds.push(c),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(_) => return Ok(End::Quit),
                }
            }
            // Quick EQ clicks queue behind a write: only the last pick is sent. A save with changes is kept.
            let picks: Vec<bool> = cmds.iter().map(|c| self.eq_pick(c)).collect();
            let last_pick = picks.iter().rposition(|&p| p);
            for (i, cmd) in cmds.into_iter().enumerate() {
                if picks[i] && Some(i) != last_pick { continue; }
                match cmd {
                    Cmd::Anc(mode) => {
                        // A mode this model does not list is never sent: its bit would be another mode's.
                        let Some(bit) = self.s.modes.bit(&mode) else { continue };
                        if LEVELS.contains(&mode.as_str()) { self.level = Some(mode.clone()); }
                        self.tx(cmd::anc_set(bit))?;
                        self.s.anc = Some(mode);
                    }
                    Cmd::LowLatency(on) => {
                        self.tx(cmd::feature(self.s.caps.game_mode_id(), on))?;
                        self.s.low_latency = Some(on);
                    }
                    // Each EQ write updates the state first, so the replies before the new values
                    // do not snap the UI back.
                    Cmd::EqBuiltIn(id) => {
                        self.s.eq_current = Some(id);
                        self.eq_write(cmd::eq_select(id))?;
                    }
                    Cmd::EqSave(p) => {
                        self.s.eq_current = Some(p.id);
                        if let Some(x) = self.s.eq_custom.iter_mut().find(|x| x.id == p.id) { *x = p.clone(); }
                        self.eq_write((CMD_SAVE_CUSTOM_EQ, p.encode(EQ_SAVE)))?;
                    }
                    // No local update: the buds assign and renumber ids.
                    Cmd::EqCreate(p) => self.eq_write((CMD_SAVE_CUSTOM_EQ, p.encode(EQ_CREATE)))?,
                    Cmd::EqDelete(p) => self.eq_write((CMD_SAVE_CUSTOM_EQ, p.encode(EQ_DELETE)))?,
                    Cmd::BassWave(on) => {
                        self.s.bass_on = Some(on);
                        self.tx(cmd::feature(FEATURE_BASSWAVE, on))?;
                    }
                    Cmd::BassLevel(l) => {
                        self.s.bass_level = Some(l);
                        self.eq_write(cmd::bass_set(l))?;
                    }
                    // As `setFeatures`: the write, then the status read that shows what the buds kept.
                    // Power saving restarts the buds: the link drops and the idle rescan reconnects.
                    Cmd::Features(changes) => {
                        for &(id, on) in &changes {
                            if let Some(x) = self.s.features.iter_mut().find(|x| x.0 == id) { x.1 = on as u8; }
                        }
                        (self.emit)(self.s.clone());
                        for (id, on) in changes {
                            self.tx(cmd::feature(id, on))?;
                            self.pump(100)?;
                        }
                        self.pump(300)?;
                        self.send(CMD_QUERY_STATUS, Some(0x00), &status_query(self.s.model))?;
                    }
                    Cmd::Find(on) => self.tx(cmd::find(on))?,
                    Cmd::FitTest(on) => self.tx(cmd::fit_test(on))?,
                    Cmd::PncQuery => self.tx(cmd::personal_anc_stored())?,
                    Cmd::Pnc(action) => {
                        self.tx(cmd::personal_anc(action))?;
                        if action == 2 { self.pump(600)?; self.send(CMD_QUERY_STATUS, Some(0x00), &status_query(self.s.model))?; }
                    }
                    Cmd::StatusRead => self.send(CMD_QUERY_STATUS, Some(0x00), &status_query(self.s.model))?,
                    // HeyMelody's order: the switch, the status re-read, then the follow-up; the buds push the list.
                    Cmd::Dual(on) => {
                        if let Some(x) = self.s.features.iter_mut().find(|x| x.0 == FEATURE_DUAL) { x.1 = on as u8; }
                        (self.emit)(self.s.clone());
                        self.tx(cmd::feature(FEATURE_DUAL, on))?;
                        self.pump(250)?;
                        self.send(CMD_QUERY_STATUS, Some(0x00), &status_query(self.s.model))?;
                        self.pump(250)?;
                        self.tx(cmd::phone_state(cmd::PhoneState { auto_switch: Some(!on), ..Default::default() }))?;
                    }
                    Cmd::DualReads(preferred) => {
                        self.tx(cmd::devices_read())?;
                        self.pump(120)?;
                        self.send(CMD_QUERY_STATUS, Some(0x00), &status_query(self.s.model))?;
                        if preferred && self.s.caps.supports(CMD_QUERY_PREFERRED) {
                            self.pump(120)?;
                            self.tx(cmd::preferred_read())?;
                        }
                    }
                    Cmd::ConnectDevice(mac, on) => {
                        self.tx(cmd::device_connect(mac, on))?;
                    }
                    Cmd::Preferred(mac) => {
                        self.tx(cmd::preferred_set(mac))?;
                        self.pump(250)?;
                        self.tx(cmd::preferred_read())?;
                    }
                    // As `writeThenRead`: shown at once, then read back.
                    Cmd::GameSoundType(t) => {
                        if let Some(g) = self.s.game_sound.as_mut() { g.0 = t; }
                        (self.emit)(self.s.clone());
                        self.tx(cmd::game_sound(t))?;
                        self.pump(400)?;
                        self.tx(cmd::game_sound_read())?;
                    }
                    Cmd::SpatialType(t) => {
                        self.s.spatial_type = Some(t);
                        (self.emit)(self.s.clone());
                        self.tx(cmd::spatial(t))?;
                        self.pump(400)?;
                        self.tx(cmd::spatial_read())?;
                    }
                    Cmd::Codec(c, hires) => self.tx(cmd::codec(c, hires))?,
                    Cmd::HeadMotion(t) => {
                        self.s.head_motion = Some(t);
                        (self.emit)(self.s.clone());
                        self.tx(cmd::head_gesture(t))?;
                        self.pump(400)?;
                        self.tx(cmd::head_gesture_read())?;
                    }
                    Cmd::ModelPicked => pick_model(&mut self.s),
                    Cmd::EarbudReads => for cmd in [CMD_QUERY_ALERT_VOLUME, CMD_QUERY_TAP_LEVEL, CMD_QUERY_GAME_SOUND, CMD_QUERY_HEAD_MOTION] {
                        if !self.s.caps.supports(cmd) { continue; }
                        self.send(cmd, None, &[])?;
                        self.pump(120)?;
                    },
                    // Sent on release only, so the buds play one prompt per change; the ack carries the level.
                    Cmd::AlertVolume(l) => {
                        self.s.alert_volume = Some(l);
                        self.tx(cmd::alert_volume(l.clamp(1, 10)))?;
                    }
                    // Unverified on hardware: read back, so the slider shows what the buds kept.
                    Cmd::TapLevel(l) => {
                        if let Some(t) = self.s.tap_level.as_mut() { t.0 = l; }
                        self.tx(cmd::tap_level(l.clamp(1, 5)))?;
                        self.pump(250)?;
                        self.tx(cmd::tap_level_read())?;
                    }
                    Cmd::ControlsReads(types) => {
                        if !self.s.caps.supports(CMD_QUERY_KEY_FUNCTION) { continue; }
                        self.send(CMD_QUERY_KEY_FUNCTION, None, &[])?;
                        for t in types { self.pump(120)?; self.tx(cmd::anc_read(2, t))?; }
                    }
                    // The whole table goes back with this bud's slots changed; adopted at once so a second
                    // write before the read-back does not undo the first. No table read: nothing is sent.
                    Cmd::Gesture(dev, act, f) => {
                        let Some(table) = self.s.keyfn.as_mut() else { continue };
                        let mut hit = false;
                        for e in table.iter_mut().filter(|e| e[0] == dev && e[2] == act && e[1] != BUTTON_ON_CALL) { e[3] = f; hit = true; }
                        if !hit { continue; }
                        let p = cmd::gesture_table(&table.iter().map(|e| crate::msg::Binding { dev: e[0], button: e[1], act: e[2], func: e[3] }).collect::<Vec<_>>());
                        (self.emit)(self.s.clone());
                        self.tx(p)?;
                        self.pump(600)?;
                        self.send(CMD_QUERY_KEY_FUNCTION, None, &[])?;
                    }
                    Cmd::HoldModes(mask, t) => {
                        self.tx(cmd::hold_cycle(t, mask))?;
                        self.pump(400)?;
                        self.tx(cmd::anc_read(2, t))?;
                    }
                    Cmd::OnCall(btn, act, f) => {
                        self.tx(cmd::gesture_table(&[crate::msg::Binding { dev: 4, button: btn, act, func: f }]))?;
                        self.pump(400)?;
                        self.send(CMD_QUERY_KEY_FUNCTION, None, &[])?;
                    }
                    Cmd::Hearing(frames) => {
                        let switch = frames.iter().any(|f| f.0 == CMD_SET_FEATURE);
                        for (cmd, p) in frames {
                            self.send(cmd, None, &p)?;
                            self.pump(60)?;
                        }
                        if switch { self.pump(400)?; self.send(CMD_QUERY_STATUS, Some(0x00), &status_query(self.s.model))?; }
                    }
                    Cmd::Disconnect => return Ok(End::UserDisconnect),
                    Cmd::Connect => continue,
                }
                (self.emit)(self.s.clone());
            }
        }
    }
}
