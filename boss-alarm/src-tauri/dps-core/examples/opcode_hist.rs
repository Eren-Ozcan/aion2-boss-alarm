//! Counts what a recording holds, per flow: frames, bytes the framing could not
//! place (a sign of lost TCP data), bundles, and a histogram of packet opcodes.
//!
//!     cargo run -p dps-core --example opcode_hist -- path/to/stream.bin

use std::collections::HashMap;

use dps_core::capture::framing::{decompress_bundle, walk, walk_inner, FrameKind};

fn main() {
    let path = std::env::args().nth(1).expect("usage: opcode_hist <stream.bin>");
    let data = std::fs::read(&path).expect("cannot read stream file");
    let mut flows: HashMap<u64, Vec<u8>> = HashMap::new();
    for (flow, bytes) in dps_core::recording::read_frames(&data) {
        flows.entry(flow).or_default().extend_from_slice(bytes);
    }
    for (flow, buf) in &flows {
        let framing = walk(buf);
        let mut packets = 0usize;
        let mut bundles = 0usize;
        let mut bad_bundles = 0usize;
        let mut framed = 0usize;
        let mut ops: HashMap<[u8; 2], usize> = HashMap::new();
        let mut count = |payload: &[u8], ops: &mut HashMap<[u8; 2], usize>| {
            if payload.len() >= 2 {
                *ops.entry([payload[0], payload[1]]).or_default() += 1;
            }
        };
        for f in &framing.frames {
            framed += f.len();
            match f.kind {
                FrameKind::Packet => {
                    packets += 1;
                    count(f.payload(buf), &mut ops);
                }
                FrameKind::Bundle => {
                    bundles += 1;
                    match decompress_bundle(f.payload(buf)) {
                        Some(inner) => {
                            let w = walk_inner(&inner);
                            for g in &w.frames {
                                packets += 1;
                                count(g.payload(&inner), &mut ops);
                            }
                        }
                        None => bad_bundles += 1,
                    }
                }
            }
        }
        println!(
            "flow {flow}: {} bytes, {} frames ({} bundles, {} undecodable), {packets} packets, bytes outside frames {}",
            buf.len(),
            framing.frames.len(),
            bundles,
            bad_bundles,
            buf.len().saturating_sub(framed)
        );
        let mut top: Vec<_> = ops.into_iter().collect();
        top.sort_by(|a, b| b.1.cmp(&a.1));
        for (op, n) in top.into_iter().take(14) {
            println!("   {:02x} {:02x}  x{n}", op[0], op[1]);
        }
    }
}
