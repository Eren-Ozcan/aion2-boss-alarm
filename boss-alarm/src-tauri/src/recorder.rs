//! Optional recording of the raw game stream, for checking the DPS numbers.
//!
//! Off by default and local only: the file holds whatever the server sent,
//! including chat and account data, so it is never uploaded or committed.
//! Replaying it through `DpsEngine` reproduces the meter's numbers exactly.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub struct Recorder {
    out: Option<BufWriter<File>>,
    path: Option<PathBuf>,
    bytes: u64,
}

impl Recorder {
    /// Start a new stream file under `dir`; returns its path.
    pub fn start(&mut self, dir: &Path) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let path = dir.join(format!("stream_{secs}.bin"));
        let mut out = BufWriter::new(File::create(&path)?);
        out.write_all(dps_core::recording::MAGIC)?;
        self.out = Some(out);
        self.path = Some(path.clone());
        self.bytes = 0;
        Ok(path)
    }

    pub fn write(&mut self, flow: u64, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        if let Some(out) = self.out.as_mut() {
            if dps_core::recording::write_frame(out, flow, data).is_err() {
                self.out = None;
            } else {
                self.bytes += data.len() as u64;
            }
        }
    }

    /// Close the file and write `notes` next to it; returns the stream path.
    pub fn stop(&mut self, notes: &str) -> Option<(PathBuf, u64)> {
        if let Some(mut out) = self.out.take() {
            let _ = out.flush();
        }
        let path = self.path.take()?;
        let _ = std::fs::write(path.with_extension("txt"), notes);
        Some((path, self.bytes))
    }
}
