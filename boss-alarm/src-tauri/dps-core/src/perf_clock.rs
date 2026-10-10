//! The machine's performance counter next to the wall clock, for ping timing.
//!
//! The newer game client stamps its ping requests with this counter, so reading
//! the same one lets a reply be timed exactly. Same idea and shape as the one
//! in A2Tools DPS Meter (GPL-3.0). Windows only; elsewhere there is no counter
//! and ping falls back to pairing replies with requests.

use crate::combat::ping_tracker::PerfClock;

#[cfg(windows)]
pub fn perf_clock() -> Option<PerfClock> {
    Some(read)
}

#[cfg(not(windows))]
pub fn perf_clock() -> Option<PerfClock> {
    None
}

/// The counter in ms since boot and the wall clock in Unix ms, read together.
#[cfg(windows)]
fn read() -> (i64, i64) {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn QueryPerformanceCounter(count: *mut i64) -> i32;
        fn QueryPerformanceFrequency(frequency: *mut i64) -> i32;
    }
    let (mut count, mut frequency) = (0i64, 0i64);
    // SAFETY: both write one i64 through a valid pointer and cannot fail on
    // Windows XP or later.
    unsafe {
        QueryPerformanceFrequency(&mut frequency);
        QueryPerformanceCounter(&mut count);
    }
    let wall_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let counter_ms = (count as i128 * 1000 / frequency.max(1) as i128) as i64;
    (counter_ms, wall_ms)
}
