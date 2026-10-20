//! The selector: one link, new protocol (`newproto.rs`, `buds_core`) or legacy (`protocol.rs`). The session calls
//! this and never branches on the protocol itself.

use crate::protocol;
use crate::types::{Event, Frame};
use buds_core::wire::Reassembler;

pub enum Proto { New(Reassembler), Legacy(protocol::Framer) }

impl Proto {
    pub fn new(next: bool) -> Proto { if next { Proto::New(Default::default()) } else { Proto::Legacy(Default::default()) } }

    pub fn is_new(&self) -> bool { matches!(self, Proto::New(_)) }

    /// The bytes to write for one message.
    pub fn frames_out(&self, cmd: u16, seq: u8, payload: &[u8]) -> Vec<Vec<u8>> {
        match self {
            Proto::New(_) => crate::newproto::frames_out(cmd, seq, payload),
            Proto::Legacy(_) => vec![crate::types::build_packet(cmd, seq, payload)],
        }
    }

    /// Bytes read -> whole messages, each as a log-format frame.
    pub fn frames_in(&mut self, data: &[u8]) -> Vec<Vec<u8>> {
        match self {
            Proto::New(r) => crate::newproto::frames_in(r, data),
            Proto::Legacy(f) => f.push(data),
        }
    }

    /// Bytes skipped while looking for a frame start (for the log).
    pub fn take_discarded(&mut self) -> Vec<u8> {
        match self {
            Proto::New(r) => std::mem::take(&mut r.discarded),
            Proto::Legacy(f) => std::mem::take(&mut f.discarded),
        }
    }

    /// The new decoder first; what it does not know goes to the legacy decoder.
    pub fn decode(&self, p: &[u8]) -> Option<Event> {
        match self {
            Proto::New(_) => crate::newproto::event(p).or_else(|| protocol::decode(p)),
            Proto::Legacy(_) => protocol::decode(p),
        }
    }

    /// The reply to a request the buds send, as `((cmd, payload), seq)`.
    pub fn answer(&self, p: &[u8], now: u32) -> Option<(Frame, u8)> {
        match self {
            Proto::New(_) => crate::newproto::answer(p, now),
            Proto::Legacy(_) => protocol::answer_for(p, now).map(|a| (a, crate::types::seq_of(p))),
        }
    }
}
