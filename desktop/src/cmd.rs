//! Typed messages of docs/PROTOCOL-NEXT.md: builds what the app sends. Each function returns a [`Frame`]
//! (command, payload); the link layer adds the sequence number and the frame. Written from the spec.

use crate::msg::Bitmap;

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
}
