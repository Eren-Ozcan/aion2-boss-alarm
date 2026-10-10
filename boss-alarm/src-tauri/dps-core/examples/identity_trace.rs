//! Trace who the meter thinks you are while a recorded stream is replayed.
//!
//!     cargo run -p dps-core --example identity_trace -- path/to/stream.bin [name]

use dps_core::engine::DpsEngine;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: identity_trace <stream.bin> [name]");
    let data = std::fs::read(&path).expect("cannot read stream file");
    let mut engine = DpsEngine::new(std::env::temp_dir().join("dps-identity-trace"));
    if let Some(name) = args.next() {
        engine.set_character_name(&name);
    }
    let mut last = None;
    let mut chunk_no = 0usize;
    for (flow, bytes) in dps_core::recording::read_frames(&data) {
        for chunk in bytes.chunks(1400) {
            engine.feed(flow, chunk);
            chunk_no += 1;
            if chunk_no % 20 == 0 {
                let _ = engine.snapshot();
            }
            let state = engine.identity_state();
            if last.as_ref() != Some(&state) {
                println!("chunk {chunk_no:6}: id={:?} scope_only={} from_game={} name={:?}", state.0, state.1, state.2, state.3);
                last = Some(state);
            }
        }
    }
}
