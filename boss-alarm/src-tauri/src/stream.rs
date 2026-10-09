//! TCP stream reassembly and game-connection detection.
//!
//! Every captured flow is reassembled in sequence order and probed with the
//! game framing. A flow becomes "the game" the first time it yields a server
//! clock packet (`00 36`, sent many times a second); flows that never do are
//! dropped after a probing budget, so browsers and downloads cost little.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::capture::Segment;
use crate::parser::{server_time_ms, split_packets};

/// How much of a flow to read before giving up on it being the game.
const PROBE_BUDGET_BYTES: usize = 192 * 1024;
/// Out-of-order segments kept while waiting for a gap to fill.
const MAX_PENDING: usize = 256;
/// Unconsumed bytes of the game flow beyond which it has lost sync.
const MAX_GAME_BUFFER: usize = 256 * 1024;

type FlowKey = ([u8; 4], u16, [u8; 4], u16);

#[derive(Default)]
struct Flow {
    buf: Vec<u8>,
    next_seq: Option<u32>,
    pending: BTreeMap<u32, Vec<u8>>,
    game: bool,
    seen_bytes: usize,
}

#[derive(Default)]
pub struct Tracker {
    flows: HashMap<FlowKey, Flow>,
    ignored: HashSet<FlowKey>,
}

/// Packet bodies decoded from one segment of the game connection.
pub struct Decoded {
    pub bodies: Vec<Vec<u8>>,
}

/// Signed distance from `b` to `a` in TCP sequence space.
fn seq_diff(a: u32, b: u32) -> i32 {
    a.wrapping_sub(b) as i32
}

impl Tracker {
    pub fn feed(&mut self, seg: Segment) -> Option<Decoded> {
        if matches!(seg.src_port, 80 | 443) || matches!(seg.dst_port, 80 | 443) {
            return None;
        }
        let key: FlowKey = (seg.src_ip, seg.src_port, seg.dst_ip, seg.dst_port);
        if self.ignored.contains(&key) {
            return None;
        }
        let flow = self.flows.entry(key).or_default();

        match flow.next_seq {
            None => {
                flow.next_seq = Some(seg.seq.wrapping_add(seg.payload.len() as u32));
                flow.buf.extend_from_slice(&seg.payload);
            }
            Some(next) => {
                let d = seq_diff(seg.seq, next);
                if d == 0 {
                    flow.next_seq = Some(next.wrapping_add(seg.payload.len() as u32));
                    flow.buf.extend_from_slice(&seg.payload);
                } else if d < 0 {
                    // Retransmission, or the same frame seen on a second device.
                    let skip = (-d) as usize;
                    if skip >= seg.payload.len() {
                        return None;
                    }
                    flow.next_seq = Some(next.wrapping_add((seg.payload.len() - skip) as u32));
                    flow.buf.extend_from_slice(&seg.payload[skip..]);
                } else {
                    // A gap: hold the segment until the missing bytes arrive.
                    flow.pending.insert(seg.seq, seg.payload);
                    if flow.pending.len() > MAX_PENDING {
                        *flow = Flow::default();
                    }
                    return None;
                }
            }
        }
        // Drain held segments that are now contiguous.
        while let Some((&seq, _)) = flow.pending.iter().next() {
            let next = flow.next_seq.unwrap_or(seq);
            let d = seq_diff(seq, next);
            if d > 0 {
                break;
            }
            let data = flow.pending.remove(&seq).unwrap_or_default();
            let skip = (-d) as usize;
            if skip < data.len() {
                flow.next_seq = Some(next.wrapping_add((data.len() - skip) as u32));
                flow.buf.extend_from_slice(&data[skip..]);
            }
        }

        flow.seen_bytes += seg.payload.len();
        let (bodies, consumed) = split_packets(&flow.buf);
        flow.buf.drain(..consumed);

        if !flow.game {
            if bodies.iter().any(|b| server_time_ms(b).is_some()) {
                flow.game = true;
            } else if flow.seen_bytes > PROBE_BUDGET_BYTES {
                self.flows.remove(&key);
                self.ignored.insert(key);
                return None;
            } else {
                return None;
            }
        } else if flow.buf.len() > MAX_GAME_BUFFER {
            // Lost sync (a dropped segment): start again at the next packet.
            flow.buf.clear();
            flow.pending.clear();
            flow.next_seq = None;
        }
        Some(Decoded { bodies })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(seq: u32, payload: Vec<u8>) -> Segment {
        Segment { src_ip: [1, 2, 3, 4], src_port: 13328, dst_ip: [10, 0, 0, 2], dst_port: 50000, seq, payload }
    }

    fn ping_packet() -> Vec<u8> {
        let mut body = vec![0x00, 0x36];
        body.extend_from_slice(&1_791_563_199_268u64.to_le_bytes());
        let mut v = vec![body.len() as u8 + 4];
        v.extend(body);
        v
    }

    #[test]
    fn detects_the_game_and_survives_reordering_and_duplicates() {
        let p = ping_packet();
        let stream: Vec<u8> = [p.clone(), p.clone(), p.clone()].concat();
        let (a, rest) = stream.split_at(7);
        let (b, c) = rest.split_at(9);

        let mut t = Tracker::default();
        // Capture starts mid-flow at the first segment; the third arrives early.
        let r1 = t.feed(seg(1000, a.to_vec()));
        assert!(r1.is_none() || r1.as_ref().unwrap().bodies.len() <= 1);
        assert!(t.feed(seg(1000 + 16, c.to_vec())).is_none()); // gap, held
        let r2 = t.feed(seg(1000 + 7, b.to_vec())).expect("game found");
        assert!(!r2.bodies.is_empty());
        // A duplicate (second capture device) adds nothing.
        assert!(t.feed(seg(1000 + 7, b.to_vec())).map_or(true, |d| d.bodies.is_empty()));
    }

    #[test]
    fn ignores_web_ports() {
        let mut t = Tracker::default();
        let mut s = seg(1, ping_packet());
        s.src_port = 443;
        assert!(t.feed(s).is_none());
    }
}
