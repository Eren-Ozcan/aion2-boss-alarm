//! A small log file for support: what the app did, never what the game sent.
//! No chat, names or packet data go in here.

use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

pub const FILE_NAME: &str = "app.log";
const MAX_BYTES: u64 = 512 * 1024;

static PATH: OnceLock<PathBuf> = OnceLock::new();

/// Point the log at `dir`. A log past its size limit is restarted, with the
/// previous one kept as `app.old.log`.
pub fn init(dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join(FILE_NAME);
    if std::fs::metadata(&path).map(|m| m.len() > MAX_BYTES).unwrap_or(false) {
        let _ = std::fs::rename(&path, dir.join("app.old.log"));
    }
    let _ = PATH.set(path);
}

/// Append one timestamped line (UTC). Failures are ignored: logging must never
/// break the app.
pub fn line(msg: &str) {
    let Some(path) = PATH.get() else { return };
    let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) else { return };
    let _ = writeln!(f, "{} {}", stamp(), msg.replace(['\r', '\n'], " "));
}

fn stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    #[test]
    fn stamp_has_the_expected_shape() {
        let s = super::stamp();
        assert_eq!(s.len(), 20);
        assert!(s.ends_with('Z'));
    }
}
