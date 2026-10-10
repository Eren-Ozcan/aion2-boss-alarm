//! Packet parser and combat aggregation for the DPS meter.
//!
//! Derived from the parser core of A2Tools DPS Meter
//! (https://github.com/taengu/A2Tools-DPS-Meter, GPL-3.0, commit 82e53c1).
//! See `NOTICE.md` for what was taken and what was left out.

pub mod atomic_file;
pub mod capture;
pub mod clock;
pub mod combat;
pub mod engine;
pub mod entity;
pub mod history;
pub mod i18n;
pub mod perf_clock;
pub mod recording;
pub mod supporters;
