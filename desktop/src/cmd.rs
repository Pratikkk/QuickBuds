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
}
