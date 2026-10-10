//! Replace a file so a crash mid-write leaves the old content, not a
//! truncated file. Simplified: write a sibling temp file, then rename it over
//! the target (the A2Tools original also tunes permissions and retries the
//! rename for antivirus).

use std::io::{self, Write};
use std::path::Path;

pub fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".{}.tmp", std::process::id()));
    let tmp = std::path::PathBuf::from(tmp);
    let result = std::fs::File::create(&tmp)
        .and_then(|mut f| f.write_all(contents).and_then(|_| f.sync_all()))
        .and_then(|_| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}
