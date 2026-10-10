//! Replay a recorded stream file through the DPS engine and print the result.
//!
//!     cargo run -p dps-core --example replay -- path/to/stream_123.bin

use dps_core::engine::DpsEngine;

fn main() {
    let path = std::env::args().nth(1).expect("usage: replay <stream.bin>");
    let data = std::fs::read(&path).expect("cannot read stream file");
    let mixed = std::env::args().any(|a| a == "--mixed"); // the old behaviour: one parser for every flow
    let mut engine = DpsEngine::new(std::env::temp_dir().join("dps-replay"));
    engine.set_target_mode(if std::env::args().any(|a| a == "--boss") { "bossTargets" } else { "allTargets" });
    // Feed in modest chunks, the way TCP segments arrive, one flow at a time.
    for (flow, bytes) in dps_core::recording::read_frames(&data) {
        for chunk in bytes.chunks(1400) {
            engine.feed(if mixed { 0 } else { flow }, chunk);
        }
    }
    // Boss fights the meter would have saved to its history.
    for f in engine.all_boss_fights() {
        println!("fight: {} | {} ms | total {}", f.boss_name, f.duration_ms, f.total_damage);
    }
    let snap = engine.snapshot();
    println!("target: {}\nbattle_ms: {}", snap.target_name, snap.battle_time);
    let mut rows: Vec<_> = snap.map.values().collect();
    rows.sort_by(|a, b| b.amount.total_cmp(&a.amount));
    for p in rows {
        println!("{}\t{}\tdamage={:.0}\tdps={:.1}", p.nickname, p.job, p.amount, p.dps);
    }
}
