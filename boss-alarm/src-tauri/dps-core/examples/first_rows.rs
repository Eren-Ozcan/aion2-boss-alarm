//! How early do rows show up in each target mode? Feeds a recorded stream in
//! TCP-sized chunks and reports the chunk at which the first row appears.
//!
//!     cargo run -p dps-core --example first_rows -- path/to/stream.bin

use dps_core::engine::DpsEngine;

fn main() {
    let path = std::env::args().nth(1).expect("usage: first_rows <stream.bin>");
    let data = std::fs::read(&path).expect("cannot read stream file");
    for mode in ["bossTargets", "bossTargets+every", "lastHitByMe", "allTargets", "trainTargets"] {
        let mut engine = DpsEngine::new(std::env::temp_dir().join("dps-first-rows"));
        engine.set_target_mode(mode.trim_end_matches("+every"));
        engine.set_every_fight(mode.ends_with("+every"));
        let mut first = None;
        let mut last_total = 0.0;
        let mut growth_chunks = 0;
        let chunks: Vec<(u64, &[u8])> = dps_core::recording::read_frames(&data)
            .into_iter()
            .flat_map(|(flow, bytes)| bytes.chunks(1400).map(move |c| (flow, c)))
            .collect();
        for (i, (flow, chunk)) in chunks.iter().enumerate() {
            engine.feed(*flow, chunk);
            let snap = engine.snapshot();
            let total: f64 = snap.map.values().map(|p| p.amount).sum();
            if first.is_none() && !snap.map.is_empty() {
                first = Some((i, total, snap.target_name.clone()));
            }
            if total > last_total {
                growth_chunks += 1;
                last_total = total;
            }
        }
        let chunks = chunks.len();
        println!("{mode:18} first row at chunk {:?} of {chunks}; final total {last_total:.0}; chunks with growth {growth_chunks}", first);
    }
}
