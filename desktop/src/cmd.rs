//! Typed messages of docs/PROTOCOL-NEXT.md: builds what the app sends. Each function returns a [`Frame`]
//! (command, payload); the link layer adds the sequence number and the frame. Written from the spec.

use crate::msg::{Binding, Bitmap, EqPreset};

/// A message to send: (command, payload).
pub type Frame = (u16, Vec<u8>);

// --- Connection (section 6) ---

pub fn handshake() -> Frame { (0x0100, vec![]) }
pub fn product_id() -> Frame { (0x0103, vec![]) }
pub fn broadcast_codes() -> Frame { (0x0200, vec![]) }

/// `0x0205 <count> <codes...>`. Count first: `01 01 02 02` would mean "count 1, battery only".
pub fn register(codes: &[u8]) -> Frame { (0x0205, [&[codes.len() as u8][..], codes].concat()) }
/// `0x0201 <code>`, for buds without `0x0205` in the bitmap.
pub fn register_one(code: u8) -> Frame { (0x0201, vec![code]) }

/// The events to subscribe to: battery, wearing, ANC, plus fit test, hearing test and personalised ANC where the
/// bitmap has their commands. Kept to the codes the buds offer when they answered `0x0200`.
pub fn notify_ids(bitmap: &Bitmap, offered: Option<&[u8]>) -> Vec<u8> {
    let mut ids = vec![0x01, 0x02, 0x03];
    if bitmap.allows(0x0405) { ids.push(0x04); }
    if bitmap.allows(0x040D) { ids.push(0x08); }
    if bitmap.allows(0x0412) { ids.push(0x0B); }
    if let Some(o) = offered { ids.retain(|c| o.contains(c)); }
    ids
}

/// The id the model list uses: four colour ranges fold into one id.
pub fn fold_product(id: u32) -> u32 {
    match id {
        0x100100..=0x100102 => 0x060414,
        0x100200..=0x100202 => 0x060814,
        0x108100..=0x108102 => 0x068414,
        0x108200..=0x108202 => 0x068814,
        x => x,
    }
}

/// Six hex digits, as the model list writes ids: `065414`.
pub fn product_text(id: u32) -> String { format!("{:06X}", fold_product(id)) }

/// The device-type list the version text names: the `versionType` 2 versions in reply order, joined by dots.
pub fn firmware_text(parts: &[(u8, u8, String)]) -> Option<String> {
    let v: Vec<&str> = parts.iter().filter(|p| p.1 == 2).map(|p| p.2.as_str()).collect();
    (!v.is_empty()).then(|| v.join("."))
}

/// `0x012F`: `<count>` then `<cmd LE> <len LE> <payload>` per query.
pub fn batch(queries: &[Frame]) -> Frame {
    let mut p = vec![queries.len() as u8];
    for (cmd, pl) in queries {
        p.extend_from_slice(&cmd.to_le_bytes());
        p.extend_from_slice(&(pl.len() as u16).to_le_bytes());
        p.extend_from_slice(pl);
    }
    (0x012F, p)
}

// --- Reads ---

pub fn battery() -> Frame { (0x0106, vec![]) }
pub fn wearing() -> Frame { (0x0109, vec![]) }
pub fn firmware() -> Frame { (0x0105, vec![]) }
/// `0x010C <what> <arg>`: `01 01` current mode, `02 01` shared hold cycle (`02 03` left, `02 04` right), `04 01` the level Smart chose.
pub fn anc_read(what: u8, arg: u8) -> Frame { (0x010C, vec![what, arg]) }
pub fn gestures() -> Frame { (0x0108, vec![]) }
/// `0x010D <count> <ids>`.
pub fn features(ids: &[u8]) -> Frame { (0x010D, [&[ids.len() as u8][..], ids].concat()) }
pub fn eq_current() -> Frame { (0x010F, vec![]) }
pub fn eq_custom() -> Frame { (0x0122, vec![]) }
pub fn bass_level() -> Frame { (0x0124, vec![]) }

// --- Writes ---

/// `0x0404 01 01 <mask LE>`: one bit set, `bit / 8 + 1` mask bytes. `bit` is the model's SET slot.
pub fn anc_set(bit: u8) -> Frame {
    let mut p = vec![0; 2 + bit as usize / 8 + 1];
    p[0] = 1;
    p[1] = 1;
    p[2 + bit as usize / 8] = 1 << (bit % 8);
    (0x0404, p)
}

/// `0x0404 02 <kind> <mask LE>`: the modes the hold cycles. The mask takes the fewest bytes, 1 to 4.
pub fn hold_cycle(kind: u8, mask: u32) -> Frame {
    let n = (1..=4).rev().find(|&n| n == 1 || mask >> ((n - 1) * 8) & 0xFF != 0).unwrap();
    (0x0404, [&[2, kind][..], &mask.to_le_bytes()[..n]].concat())
}

/// `0x0401 <count> <entries>`: the whole table, every time.
pub fn gesture_table(table: &[Binding]) -> Frame {
    (0x0401, std::iter::once(table.len() as u8).chain(table.iter().flat_map(|b| [b.dev, b.button, b.act, b.func])).collect())
}

/// `0x0403 <id> <01|00>`.
pub fn feature(id: u8, on: bool) -> Frame { (0x0403, vec![id, on as u8]) }

/// `0x0406 <id>`: select a built-in preset.
pub fn eq_select(id: u8) -> Frame { (0x0406, vec![id]) }

/// What `0x0418` does with a preset.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EqAction {
    /// The buds assign the id (send `00`) and return it in the reply.
    Create = 1,
    /// The whole preset, every time: also on every drag step and every rename. It selects the preset.
    Save = 2,
    /// The selection falls back to `00`.
    Delete = 3,
}

/// `0x0418 <action> <min> <max> <id> <nameLen> <name> <bands> [<freq u16 LE> <gain s8>]...`
pub fn eq_preset(action: EqAction, p: &EqPreset) -> Frame {
    let mut out = vec![action as u8, p.range.0 as u8, p.range.1 as u8, p.id, p.name.len() as u8];
    out.extend_from_slice(p.name.as_bytes());
    out.push(p.bands.len() as u8);
    for (f, g) in &p.bands {
        out.extend_from_slice(&f.to_le_bytes());
        out.push(*g as u8);
    }
    (0x0418, out)
}

/// `0x041B FB 05 <level>`: BassWave level, -5..+5. The BassWave switch is feature `1D`.
pub fn bass_set(level: i8) -> Frame { (0x041B, vec![0xFB, 0x05, level as u8]) }

// --- Sound and device settings (section 13) ---

pub fn alert_volume_read() -> Frame { (0x0130, vec![]) }
/// `0x0427 <level>`, 1..10. Each write plays a prompt: send on slider release only.
pub fn alert_volume(level: u8) -> Frame { (0x0427, vec![level]) }
pub fn tap_level_read() -> Frame { (0x0133, vec![]) }
/// `0x042D <level>`, 1..5. A lower level triggers more easily.
pub fn tap_level(level: u8) -> Frame { (0x042D, vec![level]) }
pub fn head_gesture_read() -> Frame { (0x0134, vec![]) }
/// `0x0431 <type>`: `00` nod answers and shake declines, `01` the reverse.
pub fn head_gesture(kind: u8) -> Frame { (0x0431, vec![kind]) }
pub fn spatial_read() -> Frame { (0x012A, vec![]) }
/// `0x0422 <type>`: 0 off, 1 fixed, 2 head tracking.
pub fn spatial(kind: u8) -> Frame { (0x0422, vec![kind]) }
/// `0x041E <source>`: `00` the buds render 3D audio, `01` the phone does.
pub fn spatial_source(source: u8) -> Frame { (0x041E, vec![source]) }
pub fn codec_read() -> Frame { (0x0114, vec![]) }
pub fn codec_list_read() -> Frame { (0x0123, vec![]) }
/// `0x041A <codec> <hiRes> 00`. The Hi-Res byte is set only with LDAC (3) or LHDC V5 (8).
pub fn codec(codec: u8, hires: bool) -> Frame { (0x041A, vec![codec, (hires && (codec == 3 || codec == 8)) as u8, 0]) }
pub fn game_sound_read() -> Frame { (0x012B, vec![]) }
/// `0x0423 <type> 01`; type 0 is off.
pub fn game_sound(kind: u8) -> Frame { (0x0423, vec![kind, 1]) }
/// `0x0420 01 <state>`: a game app is in the foreground. Applies only while feature `21` is on.
pub fn game_eq(in_game: bool) -> Frame { (0x0420, vec![1, in_game as u8]) }
/// The codecs in a mask, as the app offers them: LHDC (7) is dropped when LHDC V5 (8) is offered too.
pub fn codecs_of(mask: u16) -> Vec<u8> {
    let all: Vec<u8> = (1..=8).filter(|c| mask & (1 << (c - 1)) != 0).collect();
    if all.contains(&8) { all.into_iter().filter(|&c| c != 7).collect() } else { all }
}

/// `0x0400 <01|00>`: ring both buds.
pub fn find(on: bool) -> Frame { (0x0400, vec![on as u8]) }
/// `0x0405 <01|00>`: fit test. The result is push `04`.
pub fn fit_test(on: bool) -> Frame { (0x0405, vec![on as u8]) }
/// `0x0412 <action>`: 1 test, 2 use the stored result, 3 cancel.
pub fn personal_anc(action: u8) -> Frame { (0x0412, vec![action]) }
pub fn personal_anc_stored() -> Frame { (0x011A, vec![]) }

// --- Dual connection (section 14) ---

pub fn devices_read() -> Frame { (0x0112, vec![]) }
/// `0x0429 01|02 <MAC>`: connect or disconnect a device. The MAC is in written order.
pub fn device_connect(mac: [u8; 6], on: bool) -> Frame { (0x0429, [&[if on { 1 } else { 2 }][..], &mac].concat()) }
/// `0x0429 04 00` automatic, `04 01 <MAC>` a chosen device.
pub fn preferred_set(mac: Option<[u8; 6]>) -> Frame {
    (0x0429, match mac { None => vec![4, 0], Some(m) => [&[4, 1][..], &m].concat() })
}
pub fn preferred_read() -> Frame { (0x0132, vec![2]) }

/// The phone's state for switching between devices: `<flags> 00`, then one byte per set flag in bit order.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct PhoneState {
    pub version: Option<u8>,
    pub screen_on: Option<bool>,
    pub held: Option<bool>,
    pub auto_switch: Option<bool>,
    pub music: Option<bool>,
}

/// `0x0413 <flags> 00 <values>`: flag bit 0 protocol version, 1 screen on, 2 held by hand, 3 auto switch on, 4 music playing.
pub fn phone_state(s: PhoneState) -> Frame {
    let fields = [s.version, s.screen_on.map(u8::from), s.held.map(u8::from), s.auto_switch.map(u8::from), s.music.map(u8::from)];
    let flags = fields.iter().enumerate().fold(0u8, |f, (i, v)| f | (v.is_some() as u8) << i);
    (0x0413, [&[flags, 0][..], &fields.iter().flatten().copied().collect::<Vec<_>>()].concat())
}

// --- Golden Sound (section 16) ---

/// 12 hearing values, left ears 1-6 then right ears 1-6, as `<side> <freq> <value>`.
fn hearing_info(values: &[i8; 12]) -> Vec<u8> { (0..12).flat_map(|i| [i as u8 / 6 + 1, i as u8 % 6 + 1, values[i] as u8]).collect() }

pub fn golden_record_read() -> Frame { (0x0115, vec![]) }
pub fn golden_scan_read() -> Frame { (0x011E, vec![]) }
/// `0x040D 04 <01|00> <id u32 BE>`: ear scan. The result is push `0E`.
pub fn ear_scan(on: bool, id: u32) -> Frame { (0x040D, [&[4, on as u8][..], &id.to_be_bytes()].concat()) }
/// `0x040D 02 <01|00>`: hearing test.
pub fn hearing_test(on: bool) -> Frame { (0x040D, vec![2, on as u8]) }
/// `0x040E 03 01 <side> <freq> <value>`: a tone on one bud. Send [tone_stop] before each new level.
pub fn tone(side: u8, freq: u8, value: i8) -> Frame { (0x040E, vec![3, 1, side, freq, value as u8]) }
pub fn tone_stop() -> Frame { (0x040E, vec![4]) }
/// `0x0116 0C <12 values> <id>`: the hearing filters of a result.
pub fn golden_filter(id: u32, values: &[i8; 12]) -> Frame { (0x0116, [vec![0x0C], hearing_info(values), id.to_be_bytes().to_vec()].concat()) }
/// `0x011F <len u16 LE> <scan> <id>`: the ear-scan filters.
pub fn golden_scan_filter(id: u32, scan: &[u8]) -> Frame {
    (0x011F, [&(scan.len() as u16).to_le_bytes()[..], scan, &id.to_be_bytes()].concat())
}
/// Applies a record: the description id, the record, the ear scan when there is one, then the switch on.
/// `desc` is the description id, 0 for none.
pub fn golden_apply(id: u32, name: &str, values: &[i8; 12], scan: &[u8], desc: u8) -> Vec<Frame> {
    let mut out = Vec::new();
    if desc > 0 { out.push((0x0411, vec![1, 1, 1, 0, desc])); }
    out.push((0x040E, [vec![3, 0x0C], hearing_info(values), id.to_be_bytes().to_vec(), name.as_bytes().to_vec()].concat()));
    if !scan.is_empty() {
        out.push((0x0415, [&[3][..], &(scan.len() as u16).to_le_bytes(), scan, &id.to_be_bytes()].concat()));
    }
    out.push(feature(0x0B, true));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol as old;

    #[test]
    fn connection_frames_match_the_shipping_builders() {
        assert_eq!(register(&[1, 2, 3]), (0x0205, old::register_payload(&[1, 2, 3])));
        assert_eq!(register(&[1, 2]).1, [2, 1, 2]);
        assert_eq!(register_one(4), (0x0201, vec![4]));
        let all = Bitmap::default();
        assert_eq!(notify_ids(&all, None), old::notify_ids(&old::Caps::default(), None));
        let buds4 = Bitmap::parse(&[0, 0xFF, 0x77, 0x5A, 0xEA, 0x67, 0x0E, 0x20, 0x07]).unwrap();
        let caps4 = old::Caps::parse(&[0, 0xFF, 0x77, 0x5A, 0xEA, 0x67, 0x0E, 0x20, 0x07]).unwrap();
        let offered = [1, 2, 3, 4, 8, 0x0B, 0xF1, 0xF2, 0xF3];
        assert_eq!(notify_ids(&buds4, Some(&offered)), old::notify_ids(&caps4, Some(&offered)));
        // Nord Buds 3 Pro offers battery and wear only (issue #2): nothing else is asked for.
        assert_eq!(notify_ids(&all, Some(&[2, 1])), [1, 2]);
        let queries = vec![(0x0102, vec![0xFF, 0xFF]), (0x0106, vec![]), (0x010C, vec![1, 1])];
        assert_eq!(batch(&queries).1, old::batch_payload(&queries));
    }

    #[test]
    fn product_ids_fold_like_the_shipping_decoder() {
        for id in [0x065414u32, 0x100100, 0x100102, 0x100103, 0x108201, 0x060C12] {
            let b = id.to_le_bytes();
            assert_eq!(Some(product_text(id)), old::product_id(&[0, b[0], b[1], b[2]]), "{id:06X}");
        }
        assert_eq!(product_text(0x065414), "065414");
    }

    #[test]
    fn firmware_joins_the_version_type_2_entries() {
        let parts: Vec<(u8, u8, String)> = vec![(1, 2, "138".into()), (2, 2, "138".into()), (3, 1, "01".into()), (3, 2, "105".into())];
        assert_eq!(firmware_text(&parts).as_deref(), Some("138.138.105"));
        assert_eq!(firmware_text(&[(3, 1, "01".into())]), None);
    }
    #[test]
    fn writes_match_the_shipping_builders() {
        // ANC, from the Buds 4 capture: Off, ANC, Transparency, Deep, Adaptive (bit 11).
        for (bit, bytes) in [(0u8, vec![1, 1, 1]), (1, vec![1, 1, 2]), (2, vec![1, 1, 4]), (4, vec![1, 1, 0x10]), (11, vec![1, 1, 0, 8])] {
            assert_eq!(anc_set(bit), (0x0404, bytes));
            assert_eq!(anc_set(bit).1, old::anc_payload(bit));
        }
        for (kind, mask) in [(1u8, 0x03u32), (1, 0x0807), (3, 0x01), (4, 0x0100), (1, 0x01020304), (1, 0)] {
            assert_eq!(hold_cycle(kind, mask).1, old::hold_modes_payload(mask, kind));
        }
        assert_eq!(hold_cycle(1, 3).1, [2, 1, 3]);
        let table = [Binding { dev: 1, button: 1, act: 2, func: 0x11 }, Binding { dev: 4, button: 6, act: 6, func: 0x1C }];
        assert_eq!(gesture_table(&table).1, old::key_function_payload(&[[1, 1, 2, 0x11], [4, 6, 6, 0x1C]]));
        // Call-control capture: `0401 01 04 06 06 1C`.
        assert_eq!(gesture_table(&table[1..]), (0x0401, vec![1, 4, 6, 6, 0x1C]));
        assert_eq!(feature(0x1D, true), (0x0403, vec![0x1D, 1]));
        assert_eq!(eq_select(2), (0x0406, vec![2]));
        assert_eq!(bass_set(-3), (0x041B, vec![0xFB, 5, 0xFD]));
        assert_eq!(features(&[5, 4, 0x0B]).1, [3, 5, 4, 0x0B]);
        assert_eq!(anc_read(1, 1), (0x010C, vec![1, 1]));
    }

    #[test]
    fn custom_preset_bytes_match_the_shipping_encoder() {
        let p = EqPreset { id: 5, name: "Mine".into(), selected: false, range: (-6, 6), bands: vec![(62, -2), (250, 0), (1000, 3)] };
        let mut o = old::Preset::new("Mine", vec![62, 250, 1000]);
        o.id = 5;
        o.gains = vec![-2, 0, 3];
        for (a, b) in [(EqAction::Create, old::EQ_CREATE), (EqAction::Save, old::EQ_SAVE), (EqAction::Delete, old::EQ_DELETE)] {
            assert_eq!(eq_preset(a, &p).1, o.encode(b));
        }
        assert_eq!(&eq_preset(EqAction::Save, &p).1[..5], [2, 0xFA, 6, 5, 4]);
    }
    #[test]
    fn settings_match_the_shipping_builders() {
        assert_eq!(codec(8, true).1, old::codec_payload(8, true));
        assert_eq!(codec(2, true).1, old::codec_payload(2, true));
        assert_eq!(codec(3, false).1, [3, 0, 0]);
        for mask in [0x0146u16, 0x00FF, 0x0040, 0x0080, 0] { assert_eq!(codecs_of(mask), old::codecs_of(mask)); }
        assert_eq!(alert_volume(6), (0x0427, vec![6]));
        assert_eq!(tap_level(3), (0x042D, vec![3]));
        assert_eq!(game_sound(3), (0x0423, vec![3, 1]));
        assert_eq!(game_eq(true), (0x0420, vec![1, 1]));
        assert_eq!(spatial(2), (0x0422, vec![2]));
        assert_eq!(spatial_source(0), (0x041E, vec![0]));
        assert_eq!(head_gesture(1), (0x0431, vec![1]));
        assert_eq!(find(true), (0x0400, vec![1]));
        assert_eq!(fit_test(false), (0x0405, vec![0]));
        assert_eq!(personal_anc(2), (0x0412, vec![2]));
        assert_eq!(preferred_read(), (0x0132, vec![2]));
        let mac = [1, 2, 3, 4, 5, 6];
        assert_eq!(device_connect(mac, true).1, [1, 1, 2, 3, 4, 5, 6]);
        assert_eq!(device_connect(mac, false).1[0], 2);
        assert_eq!(preferred_set(None).1, [4, 0]);
        assert_eq!(preferred_set(Some(mac)).1, [4, 1, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn phone_state_flags_follow_the_spec() {
        // Dual connection toggle: `08 00 01` after off, `08 00 00` after on, replayed from the capture.
        assert_eq!(phone_state(PhoneState { auto_switch: Some(true), ..Default::default() }), (0x0413, vec![0x08, 0, 1]));
        assert_eq!(phone_state(PhoneState { auto_switch: Some(false), ..Default::default() }).1, [0x08, 0, 0]);
        let all = PhoneState { version: Some(1), screen_on: Some(true), held: Some(false), auto_switch: Some(true), music: Some(true) };
        assert_eq!(phone_state(all).1, [0x1F, 0, 1, 1, 0, 1, 1]);
        assert_eq!(phone_state(PhoneState::default()).1, [0, 0]);
    }

    #[test]
    fn golden_sound_frames_match_the_shipping_builders() {
        let v: [i8; 12] = [-55, -49, -41, -35, -25, -19, -11, -5, 3, 7, 13, 17];
        assert_eq!(ear_scan(true, 0x01020304), old::ear_scan(true, 0x01020304));
        assert_eq!(hearing_test(false), old::hearing_test(false));
        assert_eq!(tone(2, 3, -30), old::hearing_tone(2, 3, -30));
        assert_eq!(tone_stop(), old::hearing_tone_stop());
        assert_eq!(golden_filter(7, &v), old::hearing_filter(7, &v));
        assert_eq!(golden_scan_filter(7, &[9; 168]), old::hearing_scan_filter(7, &[9; 168]));
        assert_eq!(golden_apply(0x01020304, "2026/09/29 01:53", &v, &[9; 168], 11), old::hearing_apply(0x01020304, "2026/09/29 01:53", &v, &[9; 168], 11));
        assert_eq!(golden_apply(1, "x", &v, &[], 0), old::hearing_apply(1, "x", &v, &[], 0));
        // The capture's apply order: restore `01 01 01 00 0B`, record, scan, switch on.
        assert_eq!(golden_apply(1, "x", &v, &[9; 168], 11).iter().map(|f| f.0).collect::<Vec<_>>(), [0x0411, 0x040E, 0x0415, 0x0403]);
        assert_eq!(golden_apply(1, "x", &v, &[], 0).last(), Some(&(0x0403, vec![0x0B, 1])));
    }
}
