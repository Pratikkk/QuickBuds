//! The NEW protocol (docs/PROTOCOL.md) as the app uses it: the glue between `buds_core` (frames, messages,
//! answers) and the app's log-format frames and `Event`. The app runs the legacy link (`protocol.rs`); this glue is used by the tests only.

use crate::types::{cmd_of, payload_of, seq_of, Event};
use buds_core::msg::{decode, Anc, Inbound, Public};
use buds_core::wire::{encode, Message, Reassembler, BUDGET};

/// The message inside a log-format frame (`types::build_packet`).
fn message(p: &[u8]) -> Message { Message { cmd: cmd_of(p), seq: seq_of(p), payload: payload_of(p).to_vec() } }

/// The wire frames of one message; a message above the frame budget goes out in slices (PROTOCOL.md section 2).
pub fn frames_out(cmd: u16, seq: u8, payload: &[u8]) -> Vec<Vec<u8>> {
    encode(&Message { cmd, seq, payload: payload.to_vec() }, BUDGET)
}

/// Bytes in, messages out, each as one log-format frame: the handlers read whole frames.
pub fn frames_in(r: &mut Reassembler, data: &[u8]) -> Vec<Vec<u8>> {
    r.push(data).iter().map(|m| crate::types::build_packet(m.cmd, m.seq, &m.payload)).collect()
}

/// The event of a frame, or `None` when `buds_core` does not know it yet.
pub fn event(p: &[u8]) -> Option<Event> { to_event(&message(p)) }

/// The reply to a request the buds send, as `((cmd, payload), seq)`.
pub fn answer(p: &[u8], now: u32) -> Option<((u16, Vec<u8>), u8)> {
    buds_core::msg::answer(&message(p), now).map(|a| ((a.cmd, a.payload), a.seq))
}

/// Core messages to the app's events. The bitmap is not bridged: the session keeps `Caps`.
pub fn to_event(m: &Message) -> Option<Event> {
    use crate::types::{HearingEv, PairedDevice};
    Some(match decode(m)? {
        Inbound::ProductId(id) => Event::ProductId(buds_core::cmd::product_text(id)),
        Inbound::Firmware(f) => Event::Firmware(buds_core::cmd::firmware_text(&f.iter().map(|x| (x.0, x.1, x.2.clone())).collect::<Vec<_>>())?),
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
        Inbound::EqCustom(l) => Event::EqCustom(l.into_iter().map(|p| crate::types::Preset {
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
        Inbound::CodecMask(mask) => Event::Codecs(crate::types::codecs_of(mask)),
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
