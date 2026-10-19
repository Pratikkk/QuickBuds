//! Typed messages of docs/PROTOCOL-NEXT.md: decodes what the buds send. Written from the spec, not from
//! `protocol.rs`. Input is a [`Message`] from the link layer; output is [`Inbound`].

use crate::wire::Message;

pub const PUSH: u16 = 0x0204;
const REPLY: u16 = 0x8000;

fn le16(p: &[u8], i: usize) -> Option<u16> { Some(u16::from_le_bytes(p.get(i..i + 2)?.try_into().ok()?)) }
fn be32(p: &[u8], i: usize) -> Option<u32> { Some(u32::from_be_bytes(p.get(i..i + 4)?.try_into().ok()?)) }
/// 1 to 4 bytes, little endian.
fn le_n(b: &[u8]) -> u32 { b.iter().take(4).rev().fold(0, |v, &x| v << 8 | x as u32) }
/// A MAC on the wire is reversed; this returns it in written order.
fn mac_at(p: &[u8], i: usize) -> Option<[u8; 6]> {
    let mut m: [u8; 6] = p.get(i..i + 6)?.try_into().ok()?;
    m.reverse();
    Some(m)
}

// --- §4 Command bitmap ---

/// Commands behind each bit of the `0x8100` reply (PROTOCOL-NEXT.md §4).
const BITS: [&[u16]; 69] = [
    &[0x0105], &[0x0106], &[0x0107], &[0x0108, 0x0401, 0x0416], &[0x0109], &[0x0400], &[0x0402], &[0x0403],
    &[0x010C, 0x0404], &[0x0405], &[0x0406, 0x010F], &[0x0407], &[], &[0x0408], &[0x0409], &[], &[],
    &[0x0114], &[], &[0x040E, 0x040D, 0x0115, 0x0116], &[0x040F], &[0x0410, 0x0119], &[0x0205], &[0x0F00],
    &[], &[0x0118, 0x0411], &[0x011A, 0x0412], &[0x011C, 0x0413], &[], &[0x0112, 0x040B],
    &[0x011E, 0x011F, 0x0415], &[0x040D], &[], &[0x0121, 0x0417], &[0x0122, 0x0418], &[0x0419],
    &[0x011D, 0x0414], &[0x0123, 0x041A], &[0x0124, 0x041B], &[0x0125, 0x041C, 0x0127, 0x041D, 0x041F],
    &[0x0421, 0x0023, 0x0024, 0x0022, 0x0126, 0x0129], &[0xEF01], &[0xEF02], &[0xEF03, 0x041E], &[0x0420],
    &[0x001C], &[], &[0x0422, 0x012A], &[0xEF04], &[0x0423, 0x012B], &[], &[0x0424], &[0xEF06], &[], &[],
    &[0x0425, 0x012E, 0x0426], &[0x012F], &[0x0427, 0x0130], &[0x0131, 0x0428], &[0x0429, 0x0132], &[0x0014],
    &[0x042D, 0x0133], &[0x042E], &[0xEF07], &[0xEF08, 0x0136, 0x0433], &[0xEF09], &[0x0431, 0x0134],
    &[0x0432, 0x0135, 0x0435], &[0x0117],
];
const ALWAYS: [u16; 11] = [0x0100, 0x0101, 0x0102, 0x0103, 0x0104, 0x0106, 0x010B, 0x010D, 0x0F00, 0x0F03, 0x0F04];

/// The commands the buds accept. Before the reply, every command is allowed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bitmap(Option<Vec<u16>>);

impl Bitmap {
    /// `00` then bytes, least significant bit first. Bits past the table are ignored.
    pub fn parse(p: &[u8]) -> Option<Bitmap> {
        let [0, bytes @ ..] = p else { return None };
        if bytes.is_empty() { return None; }
        let mut cmds = ALWAYS.to_vec();
        for (bit, set) in BITS.iter().enumerate() {
            if bytes.get(bit / 8).is_some_and(|b| b >> (bit % 8) & 1 == 1) { cmds.extend_from_slice(set); }
        }
        Some(Bitmap(Some(cmds)))
    }
    pub fn allows(&self, cmd: u16) -> bool { self.0.as_ref().map_or(true, |c| c.contains(&cmd)) }
    /// The buds have answered.
    pub fn known(&self) -> bool { self.0.is_some() }
}

// --- Typed values ---

/// One battery cell: part `01` left, `02` right, `03` case.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell { pub part: u8, pub level: u8, pub charging: bool }

#[derive(Debug, Clone, PartialEq)]
pub enum Anc {
    /// The current mode as a bitmask (the report table of §6, not the SET table).
    Mode(u32),
    /// The modes the hold cycles: `kind` `01` shared, `03` left, `04` right; mask in SET bits.
    HoldCycle { kind: u8, mask: u32 },
    /// The level Smart chose, in SET bits.
    Smart(u32),
    /// `01 02 <level>`: a level report `[VENDOR]`, never seen on our buds.
    Level(u8),
}

/// One gesture binding: `dev` 1 left, 2 right, 4 both; `button`; `act`; `func`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Binding { pub dev: u8, pub button: u8, pub act: u8, pub func: u8 }

/// Both ears' biquad filters of a Golden Sound result: sample rate, left, right.
#[derive(Debug, Clone, PartialEq)]
pub struct Curves { pub rate: u32, pub left: Vec<f32>, pub right: Vec<f32> }

/// A request from the buds, which they expect to be answered (section 15).
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// `0x0500`: the phone's clock.
    Time,
    /// `0x050C`: is the foreground app on a list the buds hold.
    AppList,
    /// `0x051C`: the app's capabilities.
    Capabilities,
    /// `0x051D`: the phone's spatial mode.
    SpatialMode,
    /// `0x0501` and `0x0502`: camera state and shutter. Not answered.
    Camera(u16),
    /// A command outside the known groups.
    Unknown(u16),
}

/// One custom equaliser preset.
#[derive(Debug, Clone, PartialEq)]
pub struct EqPreset {
    pub id: u8,
    pub name: String,
    pub selected: bool,
    /// Gain range `(min, max)` as the buds send it, echoed back on a save. `FA 06` is -6..+6.
    pub range: (i8, i8),
    /// `(frequency Hz, gain dB)` per band.
    pub bands: Vec<(u16, i8)>,
}

/// One device on the dual-connection list.
#[derive(Debug, Clone, PartialEq)]
pub struct Device { pub mac: [u8; 6], pub name: String, pub connected: bool, pub flags: u8 }

impl Device {
    pub fn this_phone(&self) -> bool { self.flags & 1 != 0 }
}

/// One entry of the connected-devices push `F2`.
#[derive(Debug, Clone, PartialEq)]
pub struct Linked { pub mac: [u8; 6], pub times: u16, pub name: String }

#[derive(Debug, Clone, PartialEq)]
pub enum Public {
    AlertVolume(u8),
    /// `auto`: the buds pick the device. `mac`: the chosen handheld, written order.
    Handheld { auto: bool, mac: Option<[u8; 6]> },
    TapLevel(u8),
    /// A flag set the layout does not name.
    Other(u16),
}

/// Push `0D`. Each field is present when its flag is set and the bytes are there.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Triangle {
    pub capability: Option<u8>,
    pub version: Option<u8>,
    pub wear: Option<u8>,
    /// Bit 0 phone has no active command, bit 1 connected by the related device, bit 2 HFP busy, bit 3 A2DP busy,
    /// bit 4 the other device's auto switch is closed `[VENDOR]`.
    pub status: Option<u8>,
    pub related_name: Option<String>,
}

/// A decoded message from the buds.
#[derive(Debug, Clone, PartialEq)]
pub enum Inbound {
    // Replies (§4)
    Bitmap(Bitmap),
    /// 24-bit product id, as the buds send it.
    ProductId(u32),
    /// `(device, version type, version)`; device 1 left, 2 right, 3 case.
    Firmware(Vec<(u8, u8, String)>),
    Offered(Vec<u8>),
    /// The `0x8205` reply, after the subscription.
    Registered,
    /// A write reply (`0x84xx`): the command written, the status (`00` accepted) and the bytes after it.
    Ack { cmd: u16, status: u8, data: Vec<u8> },
    Gestures(Vec<Binding>),
    /// `0x810D`: `(feature id, value)`.
    Features(Vec<(u8, u8)>),
    EqCurrent(u8),
    EqCustom(Vec<EqPreset>),
    BassLevel(i8),
    AlertVolume(u8),
    /// `default` is absent in a push.
    TapLevel { level: u8, default: Option<u8> },
    /// `0x812A` and push `0x0510`: 0 off, 1 fixed, 2 head tracking.
    Spatial(u8),
    Codec(u8),
    /// Bit k of the mask is codec k + 1.
    CodecMask(u16),
    GameSound { selected: u8, offered: Vec<u8> },
    PersonalAncStored(bool),
    /// `None`: the buds choose.
    Preferred(Option<[u8; 6]>),
    /// `0x8115`: the Golden Sound record on the buds. `values`: left ears 1-6, then right ears 1-6.
    GoldenRecord { id: u32, name: String, values: [i8; 12] },
    /// `0x811E`: its ear-scan data.
    GoldenScan { id: u32, data: Vec<u8> },
    /// `0x8116`: the hearing filters of a result and its enhance type.
    GoldenFilter { id: u32, enhance: u8, curves: Option<Curves> },
    /// `0x811F`: the ear-scan filters of a record.
    GoldenScanFilter { id: u32, curves: Curves },
    Request(Request),
    // Pushes (§5); the same facts also arrive as replies to a query
    Battery(Vec<Cell>),
    Wear(Vec<(u8, u8)>),
    Anc(Anc),
    FitTest { left: Option<u8>, right: Option<u8> },
    /// Both candidates; the model decides which one is the state (§5, code `05`).
    GameMode { at1: Option<u8>, at3: Option<u8> },
    Devices(Vec<Device>),
    Hearing { kind: u8, status: u8 },
    ZenState(u8),
    PersonalAnc(u8),
    Triangle(Triangle),
    EarScan { id: u32, data: Vec<u8> },
    Public(Public),
    Gesture { side: u8, button: u8, act: u8, func: u8, context: u8, options: Vec<i16> },
    Linked(Vec<Linked>),
    Diagnostic(String),
    HeadGesture(u8),
    HeadTry(u8),
    /// `kind`: 0, 1 or 2 from flag bits 0-2; `flag`: bit 7.
    VoiceAssist { kind: Option<u8>, flag: bool },
    LinkSwitch { state: u8, owner: u8, reset: u8 },
    /// A push the spec names but no feature uses (`10`, `11`, `F3`, `F9`, ...): code and bytes after it.
    Unused(u8, Vec<u8>),
}

/// Decodes one message. `None`: not a message this layer knows, or a malformed one.
pub fn decode(m: &Message) -> Option<Inbound> {
    let p = &m.payload[..];
    if m.cmd == PUSH { return push(p); }
    // A write reply or a status-led reply starts with a status byte; a non-zero one carries no data.
    Some(match m.cmd {
        0x8100 => Inbound::Bitmap(Bitmap::parse(p)?),
        0x8103 => match p { [0, a, b, c] => Inbound::ProductId(u32::from_le_bytes([*a, *b, *c, 0])), _ => return None },
        0x8105 => Inbound::Firmware(firmware(p)?),
        0x8200 => match p { [0, n, codes @ ..] => Inbound::Offered(codes.get(..*n as usize)?.to_vec()), _ => return None },
        0x8205 => Inbound::Registered,
        0x8106 => Inbound::Battery(battery(ok(p)?)?),
        0x8109 => {
            // `<count> <pairs>`, or a status byte first; a list is valid when its parts are 1-3.
            let valid = |v: &Vec<(u8, u8)>| v.iter().all(|x| (1..=3).contains(&x.0));
            let list = pairs(p).filter(valid).or_else(|| pairs(p.get(1..)?).filter(valid))?;
            Inbound::Wear(list.into_iter().map(|(c, s)| (c, s & 7)).collect())
        }
        0x810C => Inbound::Anc(anc(ok(p)?)?),
        0x8108 => {
            let [0, n, e @ ..] = p else { return None };
            let b: Vec<Binding> = e.chunks_exact(4).take(*n as usize).map(|c| Binding { dev: c[0], button: c[1], act: c[2], func: c[3] }).collect();
            // A reply with no entries is no table: a write is built from the last good one.
            if b.is_empty() { return None; }
            Inbound::Gestures(b)
        }
        0x810D => Inbound::Features(pairs(ok(p)?)?),
        0x810F => Inbound::EqCurrent(*ok(p)?.first()?),
        0x8122 => Inbound::EqCustom(presets(ok(p)?)?),
        0x8124 => match ok(p)? { [_, _, level, ..] => Inbound::BassLevel(*level as i8), _ => return None },
        0x0504 => Inbound::EqCurrent(*p.first()?),
        0x8130 => Inbound::AlertVolume(*ok(p)?.first()?),
        0x8133 => match ok(p)? { [level, default, ..] => Inbound::TapLevel { level: *level, default: Some(*default) }, _ => return None },
        0x812A => Inbound::Spatial(*ok(p)?.first()?),
        0x0510 => Inbound::Spatial(*p.first()?),
        0x8134 => Inbound::HeadGesture(*ok(p)?.first()?),
        0x8114 => Inbound::Codec(*ok(p)?.first()?),
        0x8123 => Inbound::CodecMask(le16(ok(p)?, 0)?),
        0x812B => match ok(p)? {
            [selected, n, rest @ ..] => Inbound::GameSound { selected: *selected, offered: rest[..rest.len().min(*n as usize)].to_vec() },
            _ => return None,
        },
        0x811A => Inbound::PersonalAncStored(*ok(p)?.first()? != 0),
        0x8112 => Inbound::Devices(devices(ok(p)?)?),
        0x8132 => match p { [0, 2, 0, ..] => Inbound::Preferred(None), [0, 2, _, ..] => Inbound::Preferred(Some(p.get(3..9)?.try_into().ok()?)), _ => return None },
        0x8115 => golden_record(ok(p)?)?,
        0x811E => { let (id, data) = ear_scan_body(ok(p)?, 0)?; Inbound::GoldenScan { id, data } }
        0x8116 => match ok(p)? {
            d if d.len() >= 9 => Inbound::GoldenFilter { id: be32(d, 0)?, enhance: d[8], curves: curves(d, false) },
            _ => return None,
        },
        0x811F => Inbound::GoldenScanFilter { id: be32(ok(p)?, 0)?, curves: curves(ok(p)?, true)? },
        c if c & 0x8000 == 0 && c != PUSH => Inbound::Request(request(c)?),
        c if c & 0xFF00 == 0x8400 => {
            let (&status, data) = p.split_first()?;
            Inbound::Ack { cmd: c & 0x7FFF, status, data: data.to_vec() }
        }
        _ => return None,
    })
}

/// What a request from the buds is. `None`: the command is in a known group and not a request.
fn request(cmd: u16) -> Option<Request> {
    Some(match cmd {
        0x0500 => Request::Time,
        0x050C => Request::AppList,
        0x051C => Request::Capabilities,
        0x051D => Request::SpatialMode,
        0x0501 | 0x0502 => Request::Camera(cmd),
        c if ![0x01, 0x02, 0x03, 0x04, 0x05, 0x0F].contains(&(c >> 8)) => Request::Unknown(c),
        _ => return None,
    })
}

/// `<kind> <len u16 LE> <data> <id u32 BE>`, from `from`.
fn ear_scan_body(p: &[u8], from: usize) -> Option<(u32, Vec<u8>)> {
    let len = le16(p, from + 1)? as usize;
    if len == 0 { return None; }
    Some((be32(p, from + 3 + len)?, p.get(from + 3..from + 3 + len)?.to_vec()))
}

/// `03 0C <12 x (side freq value)> <id u32 BE> <name>`.
fn golden_record(d: &[u8]) -> Option<Inbound> {
    if d.len() < 2 || d[1] != 12 { return None; }
    let end = 2 + 36;
    let id = be32(d, end)?;
    let mut values = [0i8; 12];
    for c in d[2..end].chunks_exact(3) {
        if !(1..=2).contains(&c[0]) || !(1..=6).contains(&c[1]) { return None; }
        values[(c[0] as usize - 1) * 6 + c[1] as usize - 1] = c[2] as i8;
    }
    Some(Inbound::GoldenRecord { id, name: String::from_utf8_lossy(&d[end + 4..]).into_owned(), values })
}

/// Filter floats, little endian, left ear then right. `0x8116`: `<id> 04 <count LE> <packet> <type> <floats>`.
/// `0x811F`: `<id> 04 <rate LE> <count LE> <packet> <floats>`. `d` starts at the id; a count is a multiple of 12.
fn curves(d: &[u8], scan: bool) -> Option<Curves> {
    if d.len() < 10 || d[4] != 4 { return None; }
    let (rate, count, start) = if scan { (le16(d, 5)? as u32, le16(d, 7)? as usize, 10) } else { (44100, le16(d, 5)? as usize, 9) };
    if count == 0 || count % 12 != 0 { return None; }
    let f: Vec<f32> = d.get(start..start + count * 4)?.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
    Some(Curves { rate, left: f[..count / 2].to_vec(), right: f[count / 2..].to_vec() })
}

/// The bytes after a `00` status byte. A reply with another status carries no data.
fn ok(p: &[u8]) -> Option<&[u8]> { match p { [0, rest @ ..] => Some(rest), _ => None } }

/// ANC data: `01 01 <mode LE>`, `01 02 <level>`, `02 <kind> <mask LE>`, `04 01 <mask LE>`. Used by `0x810C` and push `03`.
fn anc(d: &[u8]) -> Option<Anc> {
    Some(match d {
        [1, 1, v @ ..] if !v.is_empty() => Anc::Mode(le_n(v)),
        [1, 2, level, ..] => Anc::Level(*level),
        [2, kind, m @ ..] if !m.is_empty() => Anc::HoldCycle { kind: *kind, mask: le_n(m) },
        [4, 1, m @ ..] if !m.is_empty() => Anc::Smart(le_n(m)),
        _ => return None,
    })
}

/// `<count> <flag> <min> <max> <id> <nameLen> <name> <bands> [<freq u16 LE> <gain s8>]...` per preset.
fn presets(p: &[u8]) -> Option<Vec<EqPreset>> {
    let (&n, mut rest) = p.split_first()?;
    let mut out = Vec::new();
    for _ in 0..n {
        let h = rest.get(..5)?;
        let k = h[4] as usize;
        let name = String::from_utf8_lossy(rest.get(5..5 + k)?).into_owned();
        let bands = *rest.get(5 + k)? as usize;
        let data = rest.get(6 + k..6 + k + bands * 3)?;
        out.push(EqPreset {
            id: h[3], name, selected: h[0] == 1, range: (h[1] as i8, h[2] as i8),
            bands: data.chunks_exact(3).map(|c| (u16::from_le_bytes([c[0], c[1]]), c[2] as i8)).collect(),
        });
        rest = &rest[6 + k + bands * 3..];
    }
    Some(out)
}

/// `00 <count>` then UTF-8 `device,type,version` triples joined by commas.
fn firmware(p: &[u8]) -> Option<Vec<(u8, u8, String)>> {
    let [0, _, text @ ..] = p else { return None };
    let text = String::from_utf8_lossy(text);
    let parts: Vec<&str> = text.split(',').map(str::trim).collect();
    let v: Vec<_> = parts.chunks_exact(3).filter_map(|c| Some((c[0].parse().ok()?, c[1].parse().ok()?, c[2].to_string()))).collect();
    (!v.is_empty()).then_some(v)
}

/// `<count>` then `<a> <b>` pairs. `None` when the pairs run past the end.
fn pairs(p: &[u8]) -> Option<Vec<(u8, u8)>> {
    let (&n, rest) = p.split_first()?;
    // A count of 0 is no list.
    (n > 0 && rest.len() >= n as usize * 2).then(|| rest.chunks_exact(2).take(n as usize).map(|c| (c[0], c[1])).collect())
}

fn battery(p: &[u8]) -> Option<Vec<Cell>> {
    // Level 0 means "not known" (a closed case): left out.
    Some(pairs(p)?.into_iter()
        .filter(|c| c.1 & 0x7F != 0)
        .map(|(part, raw)| Cell { part, level: raw & 0x7F, charging: raw & 0x80 != 0 })
        .collect())
}

/// Status bit 3 is a flag; the other bits are the state.
fn wear(p: &[u8]) -> Option<Vec<(u8, u8)>> { Some(pairs(p)?.into_iter().map(|(c, s)| (c, s & 7)).collect()) }

/// `<count>` then `<MAC 6> <len> <state> <flags> <nameLen> <name>`.
pub fn devices(p: &[u8]) -> Option<Vec<Device>> {
    let (&n, mut rest) = p.split_first()?;
    let mut out = Vec::new();
    for _ in 0..n {
        let h = rest.get(..10)?;
        let k = h[9] as usize;
        out.push(Device { mac: mac_at(h, 0)?, connected: h[7] == 2, flags: h[8], name: String::from_utf8_lossy(rest.get(10..10 + k)?).into_owned() });
        rest = &rest[10 + k..];
    }
    Some(out)
}

/// `<count>` then `<MAC 6> <times u16 LE> <nameLen> <name>`.
fn linked(p: &[u8]) -> Option<Vec<Linked>> {
    let (&n, mut rest) = p.split_first()?;
    let mut out = Vec::new();
    for _ in 0..n {
        let k = *rest.get(8)? as usize;
        out.push(Linked { mac: mac_at(rest, 0)?, times: le16(rest, 6)?, name: String::from_utf8_lossy(rest.get(9..9 + k)?).into_owned() });
        rest = &rest[9 + k..];
    }
    Some(out)
}

/// Ear scan: `<kind> <len u16 LE> <data> <id u32 BE>` after the code.
fn ear_scan(p: &[u8]) -> Option<(u32, Vec<u8>)> {
    let len = le16(p, 1)? as usize;
    if len == 0 { return None; }
    Some((be32(p, 3 + len)?, p.get(3..3 + len)?.to_vec()))
}

fn public(p: &[u8]) -> Option<Public> {
    // Flags u16 LE, then data. Flag bit 0 wins over bit 1, bit 1 over bit 2, bit 2 over bit 4.
    if p.len() < 3 { return None; }
    let flags = le16(p, 0)?;
    Some(if flags & 1 != 0 { Public::AlertVolume(p[2]) }
        else if flags & 2 != 0 { Public::Other(flags) }
        else if flags & 4 != 0 {
            let auto = p[2] == 0;
            Public::Handheld { auto, mac: if auto { None } else { Some(mac_at(p, 3).filter(|_| p.len() > 8)?) } }
        } else if flags & 16 != 0 { Public::TapLevel(p[2]) }
        else { Public::Other(flags) })
}

fn triangle(p: &[u8]) -> Option<Triangle> {
    let flags = le16(p, 0)?;
    let mut t = Triangle::default();
    let mut i = 2;
    if i >= p.len() { return Some(t); }
    let byte = |i: &mut usize| { let b = p.get(*i).copied(); if b.is_some() { *i += 1; } b };
    if flags & 1 != 0 { t.capability = byte(&mut i); }
    if flags & 2 != 0 { t.version = byte(&mut i); }
    if flags & 4 != 0 { t.wear = byte(&mut i); }
    if flags & 8 != 0 { t.status = byte(&mut i); }
    if flags & 16 != 0 {
        if let Some(k) = byte(&mut i) { t.related_name = p.get(i..i + k as usize).map(|n| String::from_utf8_lossy(n).into_owned()); }
    }
    Some(t)
}

/// `payload[0]` is the event code.
fn push(p: &[u8]) -> Option<Inbound> {
    let (&code, d) = p.split_first()?;
    Some(match code {
        0x01 => Inbound::Battery(battery(d)?),
        0x02 => Inbound::Wear(wear(d)?),
        0x03 => Inbound::Anc(anc(d)?),
        0x04 => {
            let r: Vec<(u8, u8)> = d.chunks_exact(2).map(|c| (c[0], c[1])).collect();
            let of = |dev| r.iter().find(|x| x.0 == dev).map(|x| x.1);
            Inbound::FitTest { left: of(1), right: of(2) }
        }
        0x05 if !d.is_empty() => Inbound::GameMode { at1: d.first().copied(), at3: d.get(2).copied() },
        0x06 => Inbound::Devices(devices(d)?),
        0x08 if d.len() >= 2 => Inbound::Hearing { kind: d[0], status: d[1] },
        0x0A if !d.is_empty() => Inbound::ZenState(d[0]),
        0x0B if !d.is_empty() => Inbound::PersonalAnc(d[0]),
        0x0D => Inbound::Triangle(triangle(d)?),
        0x0E => { let (id, data) = ear_scan(d)?; Inbound::EarScan { id, data } }
        0x0F => Inbound::Public(public(d)?),
        0xF1 if d.len() >= 5 => Inbound::Gesture {
            side: d[0], button: d[1], act: d[2], func: d[3], context: d[4],
            options: d[5..].chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect(),
        },
        0xF2 => Inbound::Linked(linked(d)?),
        0xF4 => Inbound::Diagnostic(String::from_utf8_lossy(d).into_owned()),
        0xF5 if !d.is_empty() => Inbound::HeadGesture(d[0]),
        0xF6 if !d.is_empty() => Inbound::HeadTry(d[0]),
        0xF7 if !d.is_empty() => Inbound::VoiceAssist {
            kind: [(4, 2), (2, 1), (1, 0)].into_iter().find(|(bit, _)| d[0] & bit != 0).map(|x| x.1),
            flag: d[0] & 0x80 != 0,
        },
        0xF8 if !d.is_empty() => Inbound::LinkSwitch { state: d[0] & 1, owner: d[0] >> 1 & 1, reset: d[0] >> 2 & 1 },
        // A named code with too few bytes is malformed and dropped.
        _ if !NAMED.contains(&code) => Inbound::Unused(code, d.to_vec()),
        _ => return None,
    })
}

const NAMED: &[u8] = &[1, 2, 3, 4, 5, 6, 8, 0x0A, 0x0B, 0x0D, 0x0E, 0x0F, 0xF1, 0xF2, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8];

// --- Requests from the buds (section 15) ---

/// The answer to a message the buds sent on their own, or `None` (no answer is due). `unix`: the phone's clock.
pub fn answer(m: &Message, unix: u32) -> Option<Message> {
    if m.cmd & 0x8000 != 0 { return None; }
    let payload = match decode(m) {
        Some(Inbound::Request(r)) => match r {
            Request::Time => [&[0u8][..], &unix.to_le_bytes()].concat(),
            Request::AppList => vec![0, 1, 0],
            Request::Capabilities => vec![0, 0, 0],
            // Always "invalid": the phone's spatial mode is not known here.
            Request::SpatialMode => vec![1],
            Request::Camera(_) => return None,
            Request::Unknown(_) => vec![1],
        },
        // Answered whether or not the list parses: the buds wait for it.
        _ if m.cmd == PUSH && m.payload.first() == Some(&0xF2) => vec![0, 0xF2],
        _ if m.cmd == PUSH && m.payload.first() == Some(&0xF4) => json_ack(&m.payload[1..]),
        _ => return None,
    };
    Some(Message { cmd: m.cmd | REPLY, seq: m.seq, payload })
}

/// `F4` JSON: `00 F4 {"cmd":<its cmd>}`; `01 F4` when it does not parse.
fn json_ack(json: &[u8]) -> Vec<u8> {
    if json.is_empty() { return vec![0, 0xF4]; }
    match serde_json::from_slice::<serde_json::Value>(json) {
        Ok(v) => {
            let echo = match v["cmd"].as_str() { Some(c) => serde_json::json!({ "cmd": c }), None => serde_json::json!({}) };
            [&[0u8, 0xF4][..], echo.to_string().as_bytes()].concat()
        }
        Err(_) => vec![1, 0xF4],
    }
}

// --- Batch (§4) ---

/// `0x812F`: `00 <count>`, then `<cmd LE> <len LE> <reply payload>` per answer. Each answer is the reply
/// `cmd | 0x8000` with the batch's `seq`. Stops at an answer that runs past the end.
pub fn unbatch(m: &Message) -> Vec<Message> {
    let mut out = Vec::new();
    let [0, n, rest @ ..] = &m.payload[..] else { return out };
    let mut r = rest;
    for _ in 0..*n {
        let [c0, c1, l0, l1, tail @ ..] = r else { break };
        let len = u16::from_le_bytes([*l0, *l1]) as usize;
        let Some(body) = tail.get(..len) else { break };
        out.push(Message { cmd: u16::from_le_bytes([*c0, *c1]) | REPLY, seq: m.seq, payload: body.to_vec() });
        r = &tail[len..];
    }
    out
}

// --- Bridge to the shipping session ---

/// The shipping session's event for a decoded message, where one exists. The session reads the shipping
/// `Event`; a message with no match here goes to the shipping decoder (the "new protocol" setting moves the
/// sections over one at a time). The bitmap is not bridged: the session keeps the shipping `Caps`.
pub fn to_event(m: &Message) -> Option<crate::protocol::Event> {
    use crate::protocol::{Event, HearingEv, PairedDevice};
    Some(match decode(m)? {
        Inbound::ProductId(id) => Event::ProductId(crate::cmd::product_text(id)),
        Inbound::Firmware(f) => Event::Firmware(crate::cmd::firmware_text(&f.iter().map(|x| (x.0, x.1, x.2.clone())).collect::<Vec<_>>())?),
        Inbound::Offered(o) => Event::Offered(o),
        Inbound::Battery(c) => Event::Battery(c.iter().map(|c| (c.part, c.level, c.charging)).collect()),
        Inbound::Wear(w) => Event::Wear(w),
        Inbound::Anc(Anc::Mode(raw)) => Event::AncRaw(raw),
        Inbound::Anc(Anc::Smart(mask)) => Event::SmartLevel(mask),
        Inbound::Anc(Anc::HoldCycle { kind, mask }) => Event::HoldMask(kind, mask),
        Inbound::Anc(Anc::Level(_)) => return None,
        Inbound::Gestures(g) => Event::KeyFunctions(g.iter().map(|b| [b.dev, b.button, b.act, b.func]).collect()),
        Inbound::Features(f) => Event::Features(f),
        Inbound::EqCurrent(id) => Event::EqCurrent(id),
        Inbound::BassLevel(l) => Event::BassLevel(l),
        Inbound::EqCustom(l) => Event::EqCustom(l.into_iter().map(|p| crate::protocol::Preset {
            id: p.id, name: p.name, selected: p.selected, tag: [p.range.0 as u8, p.range.1 as u8],
            freqs: p.bands.iter().map(|b| b.0).collect(), gains: p.bands.iter().map(|b| b.1).collect(),
        }).collect()),
        Inbound::FitTest { left, right } => Event::FitResult(left.unwrap_or(0xFF), right.unwrap_or(0xFF)),
        Inbound::GameMode { at1, at3 } => Event::GameMode(
            [&[5, at1.unwrap_or(0)][..], &at3.map_or(vec![], |s| vec![0, s])].concat()),
        Inbound::Devices(d) => Event::Devices(d.into_iter()
            .map(|d| PairedDevice { this_device: d.this_phone(), mac: d.mac, name: d.name, connected: d.connected }).collect()),
        Inbound::Hearing { kind, status } => Event::Hearing(HearingEv::Status(kind, status)),
        Inbound::EarScan { id, data } => Event::Hearing(HearingEv::EarScan(id, data)),
        Inbound::PersonalAnc(r) => Event::PncResult(r),
        Inbound::HeadGesture(t) => Event::HeadMotion(t),
        Inbound::Public(Public::TapLevel(l)) => Event::TapPush(l),
        Inbound::Public(Public::AlertVolume(l)) => Event::AlertVolume(l),
        Inbound::AlertVolume(l) => Event::AlertVolume(l),
        Inbound::TapLevel { level, default: Some(d) } => Event::TapLevel(level, d),
        Inbound::TapLevel { level, default: None } => Event::TapPush(level),
        Inbound::Spatial(t) => Event::SpatialType(t),
        Inbound::Codec(c) => Event::Codec(c),
        Inbound::CodecMask(mask) => Event::Codecs(crate::protocol::codecs_of(mask)),
        Inbound::GameSound { selected, offered } => Event::GameSound(selected, offered),
        Inbound::PersonalAncStored(e) => Event::PncStored(e),
        Inbound::Preferred(m) => Event::Preferred(m),
        Inbound::GoldenRecord { id, name, values } => Event::Hearing(HearingEv::Active(id, name, values)),
        Inbound::GoldenScan { id, data } => Event::Hearing(HearingEv::ActiveScan(id, data)),
        Inbound::GoldenFilter { id, enhance, curves } => Event::Hearing(HearingEv::Filter(id, enhance, curves.map(|c| (c.rate, c.left, c.right)))),
        Inbound::GoldenScanFilter { id, curves } => Event::Hearing(HearingEv::ScanCurves(id, (curves.rate, curves.left, curves.right))),
        // Write replies the session reads: the alert volume echo, the personalised-ANC ack, a refused Golden Sound write.
        Inbound::Ack { cmd: 0x0427, status: 0, data } if !data.is_empty() => Event::AlertVolume(data[0]),
        Inbound::Ack { cmd: 0x0412, status, .. } => Event::PncAck(status),
        Inbound::Ack { cmd: 0x040D | 0x040E, status, .. } if status != 0 => Event::Hearing(HearingEv::Refused(status)),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol as old;

    fn m(cmd: u16, seq: u8, payload: &[u8]) -> Message { Message { cmd, seq, payload: payload.to_vec() } }
    fn push_msg(payload: &[u8]) -> Message { m(PUSH, 0xFF, payload) }

    #[test]
    fn bitmap_matches_the_shipping_table() {
        // Buds 4 capture, Enco Buds2, and every single bit.
        let mut maps: Vec<Vec<u8>> = vec![vec![0, 0xFF, 0x77, 0x5A, 0xEA, 0x67, 0x0E, 0x20, 0x07], vec![0, 0x9B, 0x2C, 0x50, 0x80]];
        for bit in 0..72 { let mut v = vec![0u8; 10]; v[1 + bit / 8] = 1 << (bit % 8); maps.push(v); }
        for map in maps {
            let (new, old) = (Bitmap::parse(&map).unwrap(), old::Caps::parse(&map).unwrap());
            for cmd in 0..=0x0FFFu16 {
                // Old table lacks the bits added in this spec (35, 64, 67, 68 payload names): compare the rest.
                if [0x0419, 0x0136, 0x0433, 0x0432, 0x0135, 0x0435, 0x0117].contains(&cmd) { continue; }
                assert_eq!(new.allows(cmd), old.supports(cmd), "{map:02X?} cmd {cmd:04X}");
            }
        }
        assert!(Bitmap::default().allows(0x0422) && !Bitmap::default().known());
        assert!(Bitmap::parse(&[0, 0xFF, 0x77, 0x5A, 0xEA, 0x67, 0x0E, 0x20, 0x07]).unwrap().allows(0x012F));
        assert!(Bitmap::parse(&[1, 0xFF]).is_none() && Bitmap::parse(&[0]).is_none());
    }

    #[test]
    fn replies_from_the_buds4_capture() {
        assert_eq!(decode(&m(0x8103, 1, &[0, 0x14, 0x54, 0x06])), Some(Inbound::ProductId(0x065414)));
        assert_eq!(decode(&m(0x8103, 1, &[1, 0, 0, 0])), None);
        assert_eq!(decode(&m(0x8200, 1, &[0, 9, 1, 2, 3, 4, 8, 0x0B, 0xF1, 0xF2, 0xF3])), Some(Inbound::Offered(vec![1, 2, 3, 4, 8, 0x0B, 0xF1, 0xF2, 0xF3])));
        assert_eq!(decode(&m(0x8200, 1, &[0, 2, 2, 1])), Some(Inbound::Offered(vec![2, 1])));
        let fw = [&[0u8, 4][..], b"1,2,138,2,2,138,3,1,01,3,2,105"].concat();
        let Some(Inbound::Firmware(f)) = decode(&m(0x8105, 1, &fw)) else { panic!() };
        let joined: Vec<&str> = f.iter().filter(|x| x.1 == 2).map(|x| x.2.as_str()).collect();
        assert_eq!(joined.join("."), old::firmware_version("1,2,138,2,2,138,3,1,01,3,2,105").unwrap());
        assert_eq!(decode(&m(0x8205, 3, &[1, 2, 1, 0, 2, 0])), Some(Inbound::Registered));
    }

    #[test]
    fn batch_matches_the_shipping_split() {
        let queries = vec![(0x0106, vec![]), (0x010C, vec![1, 1]), (0x010F, vec![])];
        let replies = [(0x0106u16, vec![0, 1, 1, 0x5A]), (0x010C, vec![0, 1, 1, 8, 0]), (0x010F, vec![0, 2])];
        let mut pl = vec![0, replies.len() as u8];
        for (c, p) in &replies { pl.extend_from_slice(&c.to_le_bytes()); pl.extend_from_slice(&(p.len() as u16).to_le_bytes()); pl.extend_from_slice(p); }
        let new = unbatch(&m(0x812F, 7, &pl));
        let packet = old::build_packet(0x812F, 7, &pl);
        let old_parts = old::unbatch(&packet);
        assert_eq!(new.len(), 3);
        for (n, o) in new.iter().zip(&old_parts) {
            assert_eq!((n.cmd, n.seq, &n.payload[..]), (old::cmd_of(o), old::seq_of(o), old::payload_of(o)));
        }
        // A truncated answer ends the list.
        assert_eq!(unbatch(&m(0x812F, 7, &pl[..pl.len() - 1])).len(), 2);
        let _ = queries;
    }

    #[test]
    fn pushes_match_the_shipping_decoder() {
        // (payload, what the shipping decoder makes of it)
        let wear_old = |p: &[u8]| match old::decode(&old::build_packet(PUSH, 0xFF, p)) { Some(old::Event::Wear(w)) => w, _ => panic!() };
        let p = [2, 3, 1, 7, 2, 7, 3, 4];
        assert_eq!(decode(&push_msg(&p)), Some(Inbound::Wear(wear_old(&p))));
        let p = [2, 2, 1, 0x0F, 2, 4];
        assert_eq!(decode(&push_msg(&p)), Some(Inbound::Wear(wear_old(&p))));
        // Battery: level 0 left out, charging bit, same as the shipping decoder.
        let p = [1, 3, 1, 0x64, 2, 0xE4, 3, 0];
        let Some(old::Event::Battery(b)) = old::decode(&old::build_packet(PUSH, 0xFF, &p)) else { panic!() };
        let Some(Inbound::Battery(n)) = decode(&push_msg(&p)) else { panic!() };
        assert_eq!(n.iter().map(|c| (c.part, c.level, c.charging)).collect::<Vec<_>>(), b);
        // ANC mode, smart level.
        assert_eq!(decode(&push_msg(&[3, 1, 1, 0x08, 0x00])), Some(Inbound::Anc(Anc::Mode(8))));
        assert_eq!(decode(&push_msg(&[3, 4, 1, 0x20, 0])), Some(Inbound::Anc(Anc::Smart(0x20))));
        assert!(matches!(old::decode(&old::build_packet(PUSH, 1, &[3, 4, 1, 0x20, 0])), Some(old::Event::SmartLevel(0x20))));
        // The hold cycle push: old drops it, new names it.
        assert_eq!(decode(&push_msg(&[3, 2, 1, 7])), Some(Inbound::Anc(Anc::HoldCycle { kind: 1, mask: 7 })));
        assert!(old::decode(&old::build_packet(PUSH, 1, &[3, 2, 1, 7])).is_none());
        // Fit test: a missing bud is `None`, the shipping decoder says 0xFF.
        assert_eq!(decode(&push_msg(&[4, 1, 1, 2, 1])), Some(Inbound::FitTest { left: Some(1), right: Some(1) }));
        assert_eq!(decode(&push_msg(&[4, 2, 6])), Some(Inbound::FitTest { left: None, right: Some(6) }));
        assert!(matches!(old::decode(&old::build_packet(PUSH, 1, &[4, 2, 6])), Some(old::Event::FitResult(0xFF, 6))));
        // Game mode: the same two byte positions as `game_on`.
        assert_eq!(decode(&push_msg(&[5, 1])), Some(Inbound::GameMode { at1: Some(1), at3: None }));
        assert_eq!(decode(&push_msg(&[5, 0, 0, 1])), Some(Inbound::GameMode { at1: Some(0), at3: Some(1) }));
        assert_eq!(old::game_on(&[5, 0, 0, 1], Some("062410")), Some(true));
        assert_eq!(decode(&push_msg(&[0x0B, 2])), Some(Inbound::PersonalAnc(2)));
        assert_eq!(decode(&push_msg(&[0xF5, 1])), Some(Inbound::HeadGesture(1)));
        assert_eq!(decode(&push_msg(&[0x08, 2, 5])), Some(Inbound::Hearing { kind: 2, status: 5 }));
        assert_eq!(decode(&push_msg(&[0x0F, 0x10, 0, 4])), Some(Inbound::Public(Public::TapLevel(4))));
        assert!(matches!(old::decode(&old::build_packet(PUSH, 1, &[0x0F, 0x10, 0, 4])), Some(old::Event::TapPush(4))));
    }

    #[test]
    fn dual_devices_match() {
        // `<count> <MAC reversed> <len> <state> <flags> <nameLen> <name>`
        let mut p = vec![2];
        for (mac, st, fl, name) in [([6u8, 5, 4, 3, 2, 1], 2u8, 1u8, "Phone"), ([12, 11, 10, 9, 8, 7], 0, 0, "PC")] {
            p.extend_from_slice(&mac);
            p.extend_from_slice(&[3 + name.len() as u8, st, fl, name.len() as u8]);
            p.extend_from_slice(name.as_bytes());
        }
        let Some(Inbound::Devices(n)) = decode(&push_msg(&[&[6u8][..], &p].concat())) else { panic!() };
        let o = old::parse_devices(&p).unwrap();
        assert_eq!(n.len(), o.len());
        for (a, b) in n.iter().zip(&o) {
            assert_eq!((a.mac, &a.name, a.connected, a.this_phone()), (b.mac, &b.name, b.connected, b.this_device));
        }
        assert_eq!(n[0].mac, [1, 2, 3, 4, 5, 6]);
        assert_eq!(decode(&push_msg(&[6, 1, 1, 2, 3])), None);
    }

    #[test]
    fn ear_scan_matches() {
        let data = vec![9u8; 168];
        let p = [&[0x0Eu8, 3][..], &168u16.to_le_bytes(), &data, &0x01020304u32.to_be_bytes()].concat();
        assert_eq!(decode(&push_msg(&p)), Some(Inbound::EarScan { id: 0x01020304, data: data.clone() }));
        let Some(old::Event::Hearing(old::HearingEv::EarScan(id, d))) = old::decode(&old::build_packet(PUSH, 1, &p)) else { panic!() };
        assert_eq!((id, d), (0x01020304, data));
        assert_eq!(decode(&push_msg(&[0x0E, 3, 0, 0])), None);
    }

    #[test]
    fn pushes_the_shipping_decoder_ignores() {
        // Gesture, from the capture in PROTOCOL.md §6: right hold.
        assert_eq!(decode(&push_msg(&[0xF1, 2, 1, 4, 8, 3])), Some(Inbound::Gesture { side: 2, button: 1, act: 4, func: 8, context: 3, options: vec![] }));
        assert_eq!(decode(&push_msg(&[0xF1, 1, 1])), None);
        // Connected devices: one entry.
        let p = [&[0xF2u8, 1][..], &[6, 5, 4, 3, 2, 1], &[3, 0], &[2], b"PC"].concat();
        assert_eq!(decode(&push_msg(&p)), Some(Inbound::Linked(vec![Linked { mac: [1, 2, 3, 4, 5, 6], times: 3, name: "PC".into() }])));
        assert_eq!(decode(&push_msg(b"\xF4{\"cmd\":\"x\"}")), Some(Inbound::Diagnostic("{\"cmd\":\"x\"}".into())));
        assert_eq!(decode(&push_msg(&[0x0A, 1])), Some(Inbound::ZenState(1)));
        assert_eq!(decode(&push_msg(&[0xF6, 2])), Some(Inbound::HeadTry(2)));
        assert_eq!(decode(&push_msg(&[0xF7, 0x84])), Some(Inbound::VoiceAssist { kind: Some(2), flag: true }));
        assert_eq!(decode(&push_msg(&[0xF7, 0x00])), Some(Inbound::VoiceAssist { kind: None, flag: false }));
        assert_eq!(decode(&push_msg(&[0xF8, 0b101])), Some(Inbound::LinkSwitch { state: 1, owner: 0, reset: 1 }));
        assert_eq!(decode(&push_msg(&[0xF3, 9])), Some(Inbound::Unused(0xF3, vec![9])));
        assert_eq!(decode(&push_msg(&[0x10, 1, 2])), Some(Inbound::Unused(0x10, vec![1, 2])));
    }

    #[test]
    fn public_events() {
        assert_eq!(decode(&push_msg(&[0x0F, 1, 0, 6])), Some(Inbound::Public(Public::AlertVolume(6))));
        // Flag bit 2, auto (`00`), then a chosen handheld (`01` and a reversed MAC).
        assert_eq!(decode(&push_msg(&[0x0F, 4, 0, 0])), Some(Inbound::Public(Public::Handheld { auto: true, mac: None })));
        assert_eq!(decode(&push_msg(&[0x0F, 4, 0, 1, 6, 5, 4, 3, 2, 1])), Some(Inbound::Public(Public::Handheld { auto: false, mac: Some([1, 2, 3, 4, 5, 6]) })));
        assert_eq!(decode(&push_msg(&[0x0F, 4, 0, 1, 6, 5])), None);
        assert_eq!(decode(&push_msg(&[0x0F, 1, 0])), None);
    }

    #[test]
    fn triangle_fields_follow_the_flags() {
        let t = |p: &[u8]| match decode(&push_msg(&[&[0x0Du8][..], p].concat())) { Some(Inbound::Triangle(t)) => t, x => panic!("{x:?}") };
        let full = t(&[0x1F, 0, 2, 1, 7, 0x03, 2, b'P', b'C']);
        assert_eq!(full, Triangle { capability: Some(2), version: Some(1), wear: Some(7), status: Some(3), related_name: Some("PC".into()) });
        assert_eq!(t(&[0x04, 0, 5]), Triangle { wear: Some(5), ..Default::default() });
        // Flags set, bytes missing: what is present stays.
        assert_eq!(t(&[0x03, 0, 2]), Triangle { capability: Some(2), ..Default::default() });
    }
    #[test]
    fn battery_and_wear_replies() {
        let Some(Inbound::Battery(c)) = decode(&m(0x8106, 1, &[0, 3, 1, 0x64, 2, 0x64, 3, 0x50])) else { panic!() };
        assert_eq!(c.iter().map(|c| (c.part, c.level)).collect::<Vec<_>>(), [(1, 100), (2, 100), (3, 80)]);
        // Lid closed: only the bud outside answers.
        let Some(Inbound::Battery(c)) = decode(&m(0x8106, 1, &[0, 1, 1, 0x5A])) else { panic!() };
        assert_eq!(c, [Cell { part: 1, level: 90, charging: false }]);
        assert_eq!(decode(&m(0x8106, 1, &[1, 1, 1, 0x5A])), None);
        assert_eq!(decode(&push_msg(&[1, 0])), None);
        // Wearing: with and without a leading status byte.
        assert_eq!(decode(&m(0x8109, 1, &[3, 1, 7, 2, 7, 3, 4])), Some(Inbound::Wear(vec![(1, 7), (2, 7), (3, 4)])));
        assert_eq!(decode(&m(0x8109, 1, &[0, 3, 1, 3, 2, 3, 3, 4])), Some(Inbound::Wear(vec![(1, 3), (2, 3), (3, 4)])));
        assert_eq!(decode(&m(0x8109, 1, &[3, 9, 7, 2, 7, 3, 4])), None);
        // Same bytes through the shipping decoder.
        for p in [&[3u8, 1, 7, 2, 7, 3, 4][..], &[0, 3, 1, 3, 2, 3, 3, 4]] {
            let Some(old::Event::Wear(o)) = old::decode(&old::build_packet(0x8109, 1, p)) else { panic!() };
            assert_eq!(decode(&m(0x8109, 1, p)), Some(Inbound::Wear(o)));
        }
    }

    #[test]
    fn anc_replies_and_pushes() {
        assert_eq!(decode(&m(0x810C, 1, &[0, 1, 1, 0x08, 0x00])), Some(Inbound::Anc(Anc::Mode(8))));
        assert_eq!(decode(&m(0x810C, 1, &[0, 1, 1, 0x00, 0x08])), Some(Inbound::Anc(Anc::Mode(0x0800))));
        assert_eq!(decode(&m(0x810C, 1, &[0, 4, 1, 0x40])), Some(Inbound::Anc(Anc::Smart(0x40))));
        assert_eq!(decode(&m(0x810C, 1, &[0, 2, 1, 0x07, 0x08])), Some(Inbound::Anc(Anc::HoldCycle { kind: 1, mask: 0x0807 })));
        assert_eq!(decode(&m(0x810C, 1, &[0, 1, 2, 3])), Some(Inbound::Anc(Anc::Level(3))));
        assert_eq!(decode(&m(0x810C, 1, &[0xE, 1, 1, 8, 0])), None);
        for p in [&[0u8, 1, 1, 0x80, 0][..], &[0, 4, 1, 0x20], &[0, 2, 3, 1]] {
            let o = match old::decode(&old::build_packet(0x810C, 1, p)) {
                Some(old::Event::AncRaw(v)) => (0, 0, v),
                Some(old::Event::SmartLevel(v)) => (1, 0, v),
                Some(old::Event::HoldMask(k, v)) => (2, k, v),
                x => panic!("{p:02X?} {}", x.is_some()),
            };
            let n = match decode(&m(0x810C, 1, p)) {
                Some(Inbound::Anc(Anc::Mode(v))) => (0, 0, v),
                Some(Inbound::Anc(Anc::Smart(v))) => (1, 0, v),
                Some(Inbound::Anc(Anc::HoldCycle { kind, mask })) => (2, kind, mask),
                x => panic!("{x:?}"),
            };
            assert_eq!(o, n, "{p:02X?}");
        }
        // A write refused with no bud in an ear: status 14.
        assert_eq!(decode(&m(0x8404, 1, &[0x0E])), Some(Inbound::Ack { cmd: 0x0404, status: 14, data: vec![] }));
        assert_eq!(decode(&m(0x8404, 1, &[0])), Some(Inbound::Ack { cmd: 0x0404, status: 0, data: vec![] }));
        assert_eq!(decode(&m(0x8427, 1, &[0, 6])), Some(Inbound::Ack { cmd: 0x0427, status: 0, data: vec![6] }));
    }

    #[test]
    fn gesture_table_from_the_buds4_capture() {
        // `dev=01/btn=01[01:00 02:11 03:00 04:08 06:00 05:07]`, the same for dev 02, and the on-call rows.
        let rows: Vec<[u8; 4]> = [(1u8, 1u8, vec![(1u8, 0u8), (2, 0x11), (3, 0), (4, 8), (6, 0), (5, 7)]), (1, 6, vec![(2, 0), (3, 0), (6, 0)]),
            (2, 1, vec![(1, 0), (2, 0), (3, 0), (4, 8), (6, 0), (5, 7)]), (2, 6, vec![(2, 0), (3, 0), (6, 0)])]
            .into_iter().flat_map(|(d, b, acts)| acts.into_iter().map(move |(a, f)| [d, b, a, f])).collect();
        assert_eq!(rows.len(), 18);
        let payload: Vec<u8> = [0u8, 18].into_iter().chain(rows.iter().flatten().copied()).collect();
        let Some(Inbound::Gestures(g)) = decode(&m(0x8108, 1, &payload)) else { panic!() };
        assert_eq!(g.len(), 18);
        let Some(old::Event::KeyFunctions(o)) = old::decode(&old::build_packet(0x8108, 1, &payload)) else { panic!() };
        assert_eq!(g.iter().map(|b| [b.dev, b.button, b.act, b.func]).collect::<Vec<_>>(), o);
        assert_eq!(decode(&m(0x8108, 1, &[0, 0])), None);
        // The count wins over trailing bytes.
        let Some(Inbound::Gestures(g)) = decode(&m(0x8108, 1, &[0, 1, 1, 1, 1, 0, 9, 9, 9, 9])) else { panic!() };
        assert_eq!(g.len(), 1);
    }

    #[test]
    fn feature_and_eq_replies() {
        let reply = [0, 8, 5, 0, 4, 0, 0x0B, 1, 0x11, 1, 0x18, 1, 6, 0, 0x1B, 0, 0x1D, 1];
        let Some(Inbound::Features(f)) = decode(&m(0x810D, 1, &reply)) else { panic!() };
        assert_eq!(f.len(), 8);
        assert_eq!(f[1], (4, 0));
        assert_eq!(decode(&m(0x810F, 1, &[0, 2])), Some(Inbound::EqCurrent(2)));
        assert_eq!(decode(&m(0x0504, 0xFF, &[1])), Some(Inbound::EqCurrent(1)));
        assert_eq!(decode(&m(0x8124, 1, &[0, 0xFB, 5, 0xFE])), Some(Inbound::BassLevel(-2)));
        let p = crate::protocol::Preset { id: 4, name: "Rock".into(), freqs: vec![62, 250], gains: vec![3, -1], selected: true, tag: [0xFA, 6] };
        let body = [&[0u8, 1, 1, 0xFA, 6, 4, 4][..], b"Rock", &[2, 62, 0, 3, 0xFA, 0, 0xFF]].concat();
        let Some(Inbound::EqCustom(n)) = decode(&m(0x8122, 1, &body)) else { panic!() };
        let Some(old::Event::EqCustom(o)) = old::decode(&old::build_packet(0x8122, 1, &body)) else { panic!() };
        assert_eq!((n[0].id, n[0].selected, n[0].bands.len()), (o[0].id, o[0].selected, o[0].freqs.len()));
        assert_eq!(n[0].bands, [(62, 3), (250, -1)]);
        let _ = p;
        // A bad preset list is dropped.
        assert_eq!(decode(&m(0x8122, 1, &[0, 2, 0, 0xFA, 6, 1, 1, b'a', 0])), None);
    }
    #[test]
    fn setting_replies_match_the_shipping_decoder() {
        let old_of = |cmd: u16, p: &[u8]| old::decode(&old::build_packet(cmd, 1, p));
        assert_eq!(decode(&m(0x8130, 1, &[0, 6])), Some(Inbound::AlertVolume(6)));
        assert!(matches!(old_of(0x8130, &[0, 6]), Some(old::Event::AlertVolume(6))));
        assert_eq!(decode(&m(0x8130, 1, &[1, 6])), None);
        assert_eq!(decode(&m(0x8133, 1, &[0, 2, 3])), Some(Inbound::TapLevel { level: 2, default: Some(3) }));
        assert_eq!(decode(&m(0x812A, 1, &[0, 2])), Some(Inbound::Spatial(2)));
        assert_eq!(decode(&m(0x0510, 0xFF, &[1])), Some(Inbound::Spatial(1)));
        assert_eq!(decode(&m(0x8134, 1, &[0, 1])), Some(Inbound::HeadGesture(1)));
        // Buds 4 capture: codec LHDC V5, and the mask the table misreads.
        assert_eq!(decode(&m(0x8114, 1, &[0, 8])), Some(Inbound::Codec(8)));
        assert_eq!(decode(&m(0x8123, 1, &[0, 0x46, 0x01])), Some(Inbound::CodecMask(0x0146)));
        assert_eq!(decode(&m(0x812B, 1, &[0, 3, 2, 0, 3])), Some(Inbound::GameSound { selected: 3, offered: vec![0, 3] }));
        assert_eq!(decode(&m(0x811A, 1, &[0, 1])), Some(Inbound::PersonalAncStored(true)));
        assert_eq!(decode(&m(0x8132, 1, &[0, 2, 0])), Some(Inbound::Preferred(None)));
        assert_eq!(decode(&m(0x8132, 1, &[0, 2, 1, 1, 2, 3, 4, 5, 6])), Some(Inbound::Preferred(Some([1, 2, 3, 4, 5, 6]))));
        assert!(matches!(old_of(0x8132, &[0, 2, 1, 1, 2, 3, 4, 5, 6]), Some(old::Event::Preferred(Some([1, 2, 3, 4, 5, 6])))));
        // The device list as a reply.
        let list = [&[1u8][..], &[6, 5, 4, 3, 2, 1], &[8, 2, 1, 5], b"Phone"].concat();
        let Some(Inbound::Devices(d)) = decode(&m(0x8112, 1, &[&[0u8][..], &list].concat())) else { panic!() };
        assert_eq!((d[0].mac, d[0].name.as_str(), d[0].connected), ([1, 2, 3, 4, 5, 6], "Phone", true));
        // The bridge gives the shipping session the events it reads from write replies.
        let ev = |msg: &Message| to_event(msg);
        assert!(matches!(ev(&m(0x8427, 1, &[0, 3])), Some(old::Event::AlertVolume(3))));
        assert!(matches!(ev(&m(0x8412, 1, &[15])), Some(old::Event::PncAck(15))));
        assert!(matches!(ev(&m(0x840E, 1, &[3])), Some(old::Event::Hearing(old::HearingEv::Refused(3)))));
        assert!(ev(&m(0x840D, 1, &[0])).is_none());
    }

    #[test]
    fn golden_sound_replies_match_the_shipping_decoder() {
        let v: [i8; 12] = [-55, -49, -41, -35, -25, -19, -11, -5, 3, 7, 13, 17];
        // `0x0115`: the record the apply frames write, read back.
        let apply = crate::cmd::golden_apply(0x01020304, "2026/09/29 01:53", &v, &[9; 168], 11);
        let rec = &apply[1].1;
        let reply = [&[0u8][..], rec].concat();
        assert_eq!(decode(&m(0x8115, 1, &reply)), Some(Inbound::GoldenRecord { id: 0x01020304, name: "2026/09/29 01:53".into(), values: v }));
        let Some(old::Event::Hearing(old::HearingEv::Active(id, name, vals))) = old::decode(&old::build_packet(0x8115, 1, &reply)) else { panic!() };
        assert_eq!((id, name.as_str(), vals), (0x01020304, "2026/09/29 01:53", v));
        // `0x011E`: `00 03 A8 00 <168> <id>`.
        let scan = [&[0u8, 3][..], &168u16.to_le_bytes(), &[9; 168], &7u32.to_be_bytes()].concat();
        assert_eq!(decode(&m(0x811E, 1, &scan)), Some(Inbound::GoldenScan { id: 7, data: vec![9; 168] }));
        // Filters: 12 floats, six per ear (one biquad of six numbers each side).
        let floats: Vec<u8> = (0..12).flat_map(|i| (i as f32).to_le_bytes()).collect();
        let hf = [&[0u8][..], &9u32.to_be_bytes(), &[4], &12u16.to_le_bytes(), &[0, 2], &floats].concat();
        let Some(Inbound::GoldenFilter { id, enhance, curves }) = decode(&m(0x8116, 1, &hf)) else { panic!() };
        assert_eq!((id, enhance), (9, 2));
        let c = curves.unwrap();
        assert_eq!((c.rate, c.left.len(), c.right.len(), c.right[0]), (44100, 6, 6, 6.0));
        let Some(old::Event::Hearing(old::HearingEv::Filter(oid, oen, Some(oc)))) = old::decode(&old::build_packet(0x8116, 1, &hf)) else { panic!() };
        assert_eq!((oid, oen, oc.0, oc.1, oc.2), (9, 2, 44100, c.left, c.right));
        let sf = [&[0u8][..], &9u32.to_be_bytes(), &[4], &44100u16.to_le_bytes(), &12u16.to_le_bytes(), &[0], &floats].concat();
        let Some(Inbound::GoldenScanFilter { id, curves }) = decode(&m(0x811F, 1, &sf)) else { panic!() };
        assert_eq!((id, curves.rate, curves.left.len()), (9, 44100, 6));
        let Some(old::Event::Hearing(old::HearingEv::ScanCurves(oid, oc))) = old::decode(&old::build_packet(0x811F, 1, &sf)) else { panic!() };
        assert_eq!((oid, oc.0, oc.1), (9, 44100, curves.left));
        assert_eq!(decode(&m(0x811F, 1, &[0, 0, 0, 0, 9, 3, 0, 0, 0, 0, 0])), None);
    }

    #[test]
    fn requests_from_the_buds_are_answered_like_the_shipping_code() {
        let old_answer = |cmd: u16, seq: u8, p: &[u8]| old::answer_for(&old::build_packet(cmd, seq, p), 0x6700_1234);
        let cases: Vec<(u16, Vec<u8>)> = vec![
            (0x0500, vec![0, 0, 0, 0]), (0x0500, vec![]), (0x050C, vec![]), (0x051C, vec![]), (0x051D, vec![]),
            (0x0204, vec![0xF2, 1]),
            (PUSH, [&[0xF4u8][..], br#"{"cmd":"a2dp_caton","ver":"1"}"#].concat()), (PUSH, [&[0xF4u8][..], b"not json"].concat()), (PUSH, vec![0xF4]),
            (PUSH, vec![3, 2, 3, 1, 3]), (0x0501, vec![]), (0x0502, vec![]), (0x0504, vec![1]), (0x0600, vec![]), (0x0900, vec![9]),
            (0x8105, vec![0]), (0x0F00, vec![]),
        ];
        for (cmd, p) in cases {
            let new = answer(&m(cmd, 0x14, &p), 0x6700_1234).map(|a| (a.cmd, a.payload));
            assert_eq!(new, old_answer(cmd, 0x14, &p), "{cmd:04X} {p:02X?}");
        }
        let a = answer(&m(0x0500, 0x14, &[0, 0, 0, 0]), 0x6700_1234).unwrap();
        assert_eq!((a.cmd, a.seq, a.payload), (0x8500, 0x14, vec![0, 0x34, 0x12, 0x00, 0x67]));
        // Nord Buds 3 Pro: the request has a real Seq and a payload; the answer repeats the Seq.
        assert_eq!(answer(&m(0x0500, 0x21, &[0, 0, 0, 0]), 0).unwrap().seq, 0x21);
    }
}
