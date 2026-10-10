//! Feeds a recording in TCP-sized chunks and prints what the meter would show over
//! time: target, row count, and the top rows. Run with and without `--every` to see
//! what "show bosses I am not fighting" changes.
//!
//!     cargo run -p dps-core --example watch -- stream.bin [--every] [--mode bossTargets]

use dps_core::engine::DpsEngine;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("usage: watch <stream.bin> [--every] [--mode id]");
    let every = args.iter().any(|a| a == "--every");
    let mode = args.iter().position(|a| a == "--mode").and_then(|i| args.get(i + 1)).map(String::as_str).unwrap_or("bossTargets");
    let data = std::fs::read(path).expect("cannot read stream file");
    let mut engine = DpsEngine::new(std::env::temp_dir().join("dps-watch"));
    engine.set_target_mode(mode);
    engine.set_every_fight(every);
    let chunks: Vec<(u64, &[u8])> = dps_core::recording::read_frames(&data)
        .into_iter()
        .flat_map(|(flow, bytes)| bytes.chunks(1400).map(move |c| (flow, c)))
        .collect();
    let step = (chunks.len() / 40).max(1);
    let mut last = String::new();
    for (i, (flow, chunk)) in chunks.iter().enumerate() {
        engine.feed(*flow, chunk);
        if i % step != 0 {
            continue;
        }
        let snap = engine.snapshot();
        let mut rows: Vec<_> = snap.map.values().collect();
        rows.sort_by(|a, b| b.amount.total_cmp(&a.amount));
        let line = format!(
            "{:>3}%  target {:<22} hp {}/{} rows {:>3}  local {:?}  top: {}",
            i * 100 / chunks.len(),
            snap.target_name,
            snap.target_current_hp,
            snap.target_max_hp,
            rows.len(),
            snap.local_player_id,
            rows.iter().take(3).map(|p| format!("{}={:.0}", p.nickname, p.amount)).collect::<Vec<_>>().join(", ")
        );
        if std::env::args().any(|a| a == "--targets") && i % (step * 8) == 0 {
            for t in engine.debug_targets().iter().take(5) {
                println!("      {t}");
            }
        }
        if line[5..] != last {
            println!("{line}");
            last = line[5..].to_string();
        }
    }
}
