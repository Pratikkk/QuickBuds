//! The link layer of docs/PROTOCOL-NEXT.md §2: frames, slices, reassembly. No Bluetooth, no commands.

/// One message: what a frame, or the slices of one message, carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message { pub cmd: u16, pub seq: u8, pub payload: Vec<u8> }

/// The largest frame the sender writes unless the buds report a smaller one (§2).
pub const BUDGET: usize = 660;
/// A longer `Len` is noise, not a frame.
const MAX_LEN: usize = 4096;

/// Bytes `Len` takes: LEB128, 7 bits per byte.
fn len_size(len: usize) -> usize { if len < 1 << 7 { 1 } else if len < 1 << 14 { 2 } else { 3 } }

fn push_len(out: &mut Vec<u8>, mut len: usize) {
    loop {
        let b = (len & 0x7F) as u8;
        len >>= 7;
        out.push(if len != 0 { b | 0x80 } else { b });
        if len == 0 { break; }
    }
}

/// Frame size for `data` bytes: `AA`, `Len`, `Slice 00`, `[Fsn]`, data.
fn frame_size(data: usize, sliced: bool) -> usize {
    let len = 2 + sliced as usize + data;
    1 + len_size(len) + len
}

/// The frames that carry `m`: one whole frame, or slices when the whole frame exceeds `budget`.
pub fn encode(m: &Message, budget: usize) -> Vec<Vec<u8>> {
    let mut msg = vec![m.cmd as u8, (m.cmd >> 8) as u8, m.seq, m.payload.len() as u8, (m.payload.len() >> 8) as u8];
    msg.extend_from_slice(&m.payload);
    let frame = |slice: u8, fsn: Option<u8>, data: &[u8]| {
        let mut f = vec![0xAA];
        push_len(&mut f, 2 + fsn.is_some() as usize + data.len());
        f.extend_from_slice(&[slice, 0]);
        f.extend(fsn);
        f.extend_from_slice(data);
        f
    };
    if frame_size(msg.len(), false) <= budget { return vec![frame(0, None, &msg)]; }
    // The most data a slice holds, at least 1 byte so a tiny budget still ends.
    let per = (1..=budget).rev().find(|&d| frame_size(d, true) <= budget).unwrap_or(1);
    let chunks: Vec<&[u8]> = msg.chunks(per).collect();
    let last = chunks.len() - 1;
    chunks.iter().enumerate()
        .map(|(i, c)| frame(if i == 0 { 1 } else if i == last { 3 } else { 2 }, Some(i as u8), c))
        .collect()
}

/// Turns the byte stream into messages. Noise before a frame is skipped and kept in `discarded`.
#[derive(Default)]
pub struct Reassembler { buf: Vec<u8>, slices: Option<Vec<u8>>, pub discarded: Vec<u8> }

impl Reassembler {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Message> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        loop {
            let Some(start) = self.buf.iter().position(|&b| b == 0xAA) else {
                self.discarded.append(&mut self.buf);
                break;
            };
            if start > 0 { self.discarded.extend(self.buf.drain(..start)); }
            let Some((len, n)) = read_len(&self.buf[1..]) else { break };
            if len < 2 || len > MAX_LEN {
                self.discarded.push(self.buf.remove(0));
                continue;
            }
            let end = 1 + n + len;
            if self.buf.len() < end { break; }
            let body: Vec<u8> = self.buf.drain(..end).skip(1 + n).collect();
            if let Some(m) = self.body(&body) { out.push(m); }
        }
        out
    }

    /// One frame's bytes after `Len`: `Slice 00 [Fsn] data`.
    fn body(&mut self, b: &[u8]) -> Option<Message> {
        match b[0] {
            0 => parse(&b[2..]),
            1 if b.len() >= 3 => { self.slices = Some(b[3..].to_vec()); None }
            2 | 3 if b.len() >= 3 => {
                // A middle or last slice with no first slice before it is dropped.
                let all = self.slices.as_mut()?;
                all.extend_from_slice(&b[3..]);
                if b[0] == 3 { parse(&self.slices.take()?) } else { None }
            }
            _ => None,
        }
    }
}

/// `Len` and the bytes it took, or None while incomplete.
fn read_len(b: &[u8]) -> Option<(usize, usize)> {
    let mut v = 0usize;
    for (i, &x) in b.iter().enumerate().take(3) {
        v |= ((x & 0x7F) as usize) << (7 * i);
        if x & 0x80 == 0 { return Some((v, i + 1)); }
    }
    None
}

/// `Cmd (2 LE)  Seq  PayLen (2 LE)  Payload`. A `PayLen` past the bytes present keeps the bytes present.
fn parse(m: &[u8]) -> Option<Message> {
    if m.len() < 5 { return None; }
    let n = u16::from_le_bytes([m[3], m[4]]) as usize;
    let rest = &m[5..];
    Some(Message { cmd: u16::from_le_bytes([m[0], m[1]]), seq: m[2], payload: rest[..n.min(rest.len())].to_vec() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(cmd: u16, seq: u8, n: usize) -> Message { Message { cmd, seq, payload: (0..n).map(|i| i as u8).collect() } }

    #[test]
    fn captured_frames() {
        // PROTOCOL.md §2 examples
        assert_eq!(encode(&Message { cmd: 0x0100, seq: 1, payload: vec![] }, BUDGET), [vec![0xAA, 7, 0, 0, 0, 1, 1, 0, 0]]);
        assert_eq!(encode(&Message { cmd: 0x0404, seq: 6, payload: vec![1, 1, 1] }, BUDGET),
            [vec![0xAA, 0x0A, 0, 0, 4, 4, 6, 3, 0, 1, 1, 1]]);
        // Len above 127 takes two bytes: a 124-byte payload goes out as `AA 83 01`.
        assert_eq!(&encode(&msg(0x0418, 3, 124), BUDGET)[0][..3], [0xAA, 0x83, 0x01]);
    }

    #[test]
    fn same_bytes_as_the_shipping_encoder() {
        for n in [0, 1, 3, 100, 120, 121, 122, 250, 600] {
            let m = msg(0x0403, 9, n);
            assert_eq!(encode(&m, BUDGET), [crate::protocol::build_packet(m.cmd, m.seq, &m.payload)], "payload {n}");
        }
    }

    #[test]
    fn slices_join_again() {
        for (n, budget) in [(1500, BUDGET), (600, 100), (5000, 129), (700, 660)] {
            let m = msg(0x8115, 4, n);
            let frames = encode(&m, budget);
            assert!(frames.len() > 1 || n < 660);
            assert!(frames.iter().all(|f| f.len() <= budget), "frame over budget {budget}");
            let mut r = Reassembler::default();
            let got: Vec<Message> = frames.iter().flat_map(|f| r.push(f)).collect();
            assert_eq!(got, [m], "payload {n} budget {budget}");
        }
    }

    #[test]
    fn stream_in_pieces_and_noise() {
        let a = encode(&msg(0x0106, 1, 4), BUDGET).concat();
        let b = encode(&msg(0x0204, 0xFF, 130), BUDGET).concat();
        let mut stream = vec![0x13, 0x37];
        stream.extend(&a);
        stream.extend(&b);
        let mut r = Reassembler::default();
        let mut got = Vec::new();
        for c in stream.chunks(5) { got.extend(r.push(c)); }
        assert_eq!(got, [msg(0x0106, 1, 4), msg(0x0204, 0xFF, 130)]);
        assert_eq!(r.discarded, [0x13, 0x37]);
    }

    #[test]
    fn bad_input_is_dropped() {
        let mut r = Reassembler::default();
        // A middle slice with no first slice, a bad Len, a PayLen past the data.
        assert!(r.push(&[0xAA, 6, 2, 0, 1, 1, 2, 3]).is_empty());
        assert!(r.push(&[0xAA, 0]).is_empty());
        let short = r.push(&[0xAA, 9, 0, 0, 0x06, 0x81, 1, 9, 0, 7, 7]);
        assert_eq!(short, [Message { cmd: 0x8106, seq: 1, payload: vec![7, 7] }]);
    }
}
