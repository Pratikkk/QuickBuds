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
        _ => return None,
    })
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
    (rest.len() >= n as usize * 2).then(|| rest.chunks_exact(2).take(n as usize).map(|c| (c[0], c[1])).collect())
}

fn battery(p: &[u8]) -> Vec<Cell> {
    // Level 0 means "not known" (a closed case): left out.
    pairs(p).unwrap_or_default().into_iter()
        .filter(|c| c.1 & 0x7F != 0)
        .map(|(part, raw)| Cell { part, level: raw & 0x7F, charging: raw & 0x80 != 0 })
        .collect()
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
        0x01 => Inbound::Battery(battery(d)),
        0x02 => Inbound::Wear(wear(d)?),
        0x03 => Inbound::Anc(match d {
            [1, 1, v @ ..] if v.len() >= 2 => Anc::Mode(le_n(v)),
            [2, kind, m @ ..] if !m.is_empty() => Anc::HoldCycle { kind: *kind, mask: le_n(m) },
            [4, 1, m @ ..] if !m.is_empty() => Anc::Smart(le_n(m)),
            _ => return None,
        }),
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
}
