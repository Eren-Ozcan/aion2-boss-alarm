//! Timers, alarm settings and the alarm scheduler.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::bosses::{self, BOSSES};
use crate::events::{self, EVENTS};
use crate::parser::BossList;

/// Minutes before the respawn at which to alarm; 0 is the respawn itself.
/// A boss nobody has configured alarms 15 minutes ahead.
fn default_minutes() -> Vec<u32> {
    vec![15]
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AlarmConfig {
    pub enabled: bool,
    #[serde(default = "default_minutes")]
    pub minutes: Vec<u32>,
}

impl Default for AlarmConfig {
    fn default() -> Self {
        Self { enabled: true, minutes: default_minutes() }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct StoredTimer {
    respawn_ms: Option<u64>,
    spawned: bool,
    /// Server ms the boss came up, when we saw its timer run out. None if we
    /// only ever saw it already up.
    #[serde(default)]
    spawned_at_ms: Option<u64>,
    /// Local clock when the list that set this was received.
    seen_local_ms: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct Stored {
    alarms: HashMap<u32, AlarmConfig>,
    timers: HashMap<u32, StoredTimer>,
    /// Server clock minus local clock at the last sync. Kept so timers read
    /// right after a restart, before the game connection is seen again.
    #[serde(default)]
    offset_ms: i64,
    /// Server clock's offset from UTC in hours, for the event schedules.
    /// None until the player sets it; then the default applies. (The first version
    /// stored a plain number with a different default, so it is a new key.)
    #[serde(default)]
    server_utc_offset: Option<i32>,
    /// All alarms off, from the tray, the overlay or the window.
    #[serde(default)]
    muted: bool,
    /// Open the overlay when the app starts.
    #[serde(default = "yes")]
    overlay_at_startup: bool,
}

fn yes() -> bool {
    true
}

impl Stored {
    fn server_offset(&self) -> i32 {
        self.server_utc_offset.unwrap_or(events::DEFAULT_SERVER_OFFSET_H)
    }
}

fn unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Status {
    Starting,
    NpcapMissing(String),
    CaptureFailed(String),
    /// Capturing on `devices` devices; the game connection is not seen yet.
    WaitingForGame { devices: usize },
    /// The game connection is being read.
    Connected,
}

#[derive(Clone, Serialize)]
pub struct BossView {
    pub id: u32,
    pub name: &'static str,
    pub group: &'static str,
    pub spawned: bool,
    /// When it respawns, in local unix ms, if known.
    pub respawn_local_ms: Option<i64>,
    /// Local unix ms of the list that last updated this boss.
    pub seen_local_ms: Option<u64>,
    /// When it came up, in local unix ms, if known.
    pub spawned_at_local_ms: Option<i64>,
    pub alarm: AlarmConfig,
}

#[derive(Clone, Serialize)]
pub struct EventView {
    pub id: u32,
    pub name: &'static str,
    /// "Shugo Festival" runs on `:00`; the siege on "Mon Thu Sat 21:00".
    pub rule: String,
    /// The next start, in local unix ms.
    pub next_local_ms: i64,
    pub period_ms: i64,
    pub alarm: AlarmConfig,
}

#[derive(Clone, Serialize)]
pub struct Snapshot {
    pub status: Status,
    pub bosses: Vec<BossView>,
    pub events: Vec<EventView>,
    pub server_offset_h: i32,
    pub muted: bool,
    pub overlay_at_startup: bool,
    pub last_list_local_ms: Option<u64>,
}

#[derive(Clone, Serialize)]
pub struct AlarmEvent {
    pub id: u32,
    pub name: &'static str,
    pub minutes: u32,
    /// "boss" or "event": the page plays a different sound for each.
    pub kind: &'static str,
}

pub struct Core {
    path: PathBuf,
    stored: Stored,
    status: Status,
    /// Server clock minus local clock, ms.
    offset_ms: i64,
    last_list_local_ms: Option<u64>,
    fired: HashSet<(u32, u64, u32)>,
}

impl Core {
    pub fn load(path: PathBuf) -> Self {
        let stored = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Stored>(&s).ok())
            .unwrap_or_default();
        Self {
            path,
            offset_ms: stored.offset_ms,
            stored,
            status: Status::Starting,
            last_list_local_ms: None,
            fired: HashSet::new(),
        }
    }

    fn save(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string_pretty(&self.stored) {
            let _ = std::fs::write(&self.path, s);
        }
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    /// The server clock, learned from the stream, as an offset to the local one.
    pub fn note_server_time(&mut self, server_ms: u64, local_ms: u64) {
        self.offset_ms = server_ms as i64 - local_ms as i64;
    }

    pub fn offset_ms(&self) -> i64 {
        self.offset_ms
    }

    /// A boss list arrived: record every entry's state.
    pub fn apply_list(&mut self, list: &BossList, local_ms: u64) {
        for e in &list.entries {
            if bosses::find(e.id).is_none() {
                continue; // an id this version does not know
            }
            let prev = self.stored.timers.get(&e.id);
            // The moment a boss comes up is the respawn time we last held for it;
            // keep it for as long as the boss stays up.
            let spawned_at_ms = if e.spawned {
                prev.and_then(|p| if p.spawned { p.spawned_at_ms } else { p.respawn_ms })
                    .filter(|&t| (t as i64) <= local_ms as i64 + self.offset_ms)
            } else {
                None
            };
            self.stored.timers.insert(
                e.id,
                StoredTimer { respawn_ms: e.respawn_ms, spawned: e.spawned, spawned_at_ms, seen_local_ms: local_ms },
            );
        }
        self.last_list_local_ms = Some(local_ms);
        self.stored.offset_ms = self.offset_ms;
        self.save();
    }

    pub fn set_alarm(&mut self, id: u32, cfg: AlarmConfig) {
        let mut cfg = cfg;
        cfg.minutes.sort_unstable_by(|a, b| b.cmp(a));
        cfg.minutes.dedup();
        self.stored.alarms.insert(id, cfg);
        self.save();
    }

    pub fn set_all_enabled(&mut self, enabled: bool) {
        for b in BOSSES {
            self.stored.alarms.entry(b.id).or_default().enabled = enabled;
        }
        for e in EVENTS {
            self.stored.alarms.entry(e.id).or_default().enabled = enabled;
        }
        self.save();
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.stored.muted = muted;
        self.save();
    }

    pub fn muted(&self) -> bool {
        self.stored.muted
    }

    pub fn set_overlay_at_startup(&mut self, on: bool) {
        self.stored.overlay_at_startup = on;
        self.save();
    }

    pub fn overlay_at_startup(&self) -> bool {
        self.stored.overlay_at_startup
    }

    pub fn set_server_offset(&mut self, hours: i32) {
        self.stored.server_utc_offset = Some(hours.clamp(-12, 14));
        self.save();
    }

    pub fn snapshot(&self) -> Snapshot {
        let bosses = BOSSES
            .iter()
            .map(|b| {
                let t = self.stored.timers.get(&b.id);
                BossView {
                    id: b.id,
                    name: b.name,
                    group: b.group.key(),
                    spawned: t.is_some_and(|t| t.spawned),
                    respawn_local_ms: t.and_then(|t| t.respawn_ms).map(|r| r as i64 - self.offset_ms),
                    seen_local_ms: t.map(|t| t.seen_local_ms),
                    spawned_at_local_ms: t.and_then(|t| t.spawned_at_ms).map(|r| r as i64 - self.offset_ms),
                    alarm: self.stored.alarms.get(&b.id).cloned().unwrap_or_default(),
                }
            })
            .collect();
        let now = unix_ms();
        let off = self.stored.server_offset();
        let events = EVENTS
            .iter()
            .map(|e| EventView {
                id: e.id,
                name: e.name,
                rule: e.rule.label(),
                next_local_ms: e.rule.next_at_or_after(now, off),
                period_ms: e.rule.period_ms(),
                alarm: self.stored.alarms.get(&e.id).cloned().unwrap_or_default(),
            })
            .collect();
        Snapshot { status: self.status.clone(), bosses, events, server_offset_h: off, muted: self.stored.muted, overlay_at_startup: self.stored.overlay_at_startup, last_list_local_ms: self.last_list_local_ms }
    }

    /// Alarms that are due at `local_ms`. Each (boss, respawn time, minutes)
    /// fires once; a new respawn time (the boss was killed again) re-arms it.
    pub fn due_alarms(&mut self, local_ms: u64) -> Vec<AlarmEvent> {
        let mut out = Vec::new();
        // Muted: thresholds still pass (and count as fired), nothing rings.
        let muted = self.stored.muted;
        for b in BOSSES {
            let default = AlarmConfig::default();
            let cfg = self.stored.alarms.get(&b.id).unwrap_or(&default);
            if !cfg.enabled {
                continue;
            }
            let Some(t) = self.stored.timers.get(&b.id) else { continue };
            let Some(respawn) = t.respawn_ms.filter(|_| !t.spawned) else { continue };
            let remaining_ms = respawn as i64 - (local_ms as i64 + self.offset_ms);
            for &m in &cfg.minutes {
                let key = (b.id, respawn, m);
                if self.fired.contains(&key) {
                    continue;
                }
                if remaining_ms <= m as i64 * 60_000 {
                    self.fired.insert(key);
                    // Stale: the threshold passed long ago (app was closed); stay quiet.
                    if remaining_ms > m as i64 * 60_000 - 20_000 {
                        out.push(AlarmEvent { id: b.id, name: b.name, minutes: m, kind: "boss" });
                    }
                }
            }
        }
        // Scheduled events: the next start at or just before now, so the alarm at 0 fires on the dot.
        let now = local_ms as i64;
        for e in EVENTS {
            let default = AlarmConfig::default();
            let cfg = self.stored.alarms.get(&e.id).unwrap_or(&default);
            if !cfg.enabled {
                continue;
            }
            let start = e.rule.next_at_or_after(now - 20_000, self.stored.server_offset());
            let remaining_ms = start - now;
            for &m in &cfg.minutes {
                let key = (e.id, start as u64, m);
                if self.fired.contains(&key) {
                    continue;
                }
                if remaining_ms <= m as i64 * 60_000 {
                    self.fired.insert(key);
                    if remaining_ms > m as i64 * 60_000 - 20_000 {
                        out.push(AlarmEvent { id: e.id, name: e.name, minutes: m, kind: "event" });
                    }
                }
            }
        }
        // Forget keys of timers that no longer exist.
        if self.fired.len() > 4096 {
            self.fired.clear();
        }
        if muted {
            out.clear();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ListEntry;

    fn core() -> Core {
        Core::load(std::env::temp_dir().join(format!("boss-alarm-test-{}.json", std::process::id())))
    }

    fn list(id: u32, respawn_ms: u64) -> BossList {
        BossList { list_id: 0x0456, entries: vec![ListEntry { id, spawned: false, respawn_ms: Some(respawn_ms) }] }
    }

    #[test]
    fn alarms_fire_once_at_each_threshold_and_rearm_on_a_new_timer() {
        let mut c = core();
        c.set_alarm(111003, AlarmConfig { enabled: true, minutes: vec![5, 0] });
        let t0 = 1_000_000_000_000u64;
        c.apply_list(&list(111003, t0 + 600_000), t0); // respawns in 10 min

        assert!(c.due_alarms(t0).is_empty());
        assert!(c.due_alarms(t0 + 299_000).is_empty());
        let a = c.due_alarms(t0 + 301_000); // 4m59s left
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].minutes, 5);
        assert!(c.due_alarms(t0 + 302_000).is_empty()); // not again
        let a = c.due_alarms(t0 + 601_000);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].minutes, 0);

        // Killed again: a fresh respawn time arms the alarms again.
        c.apply_list(&list(111003, t0 + 4_000_000), t0 + 700_000);
        assert_eq!(c.due_alarms(t0 + 3_710_000).len(), 1);
    }

    #[test]
    fn disabled_bosses_and_stale_thresholds_stay_quiet() {
        let mut c = core();
        let t0 = 1_000_000_000_000u64;
        c.apply_list(&list(111004, t0 + 100_000), t0);
        c.set_alarm(111004, AlarmConfig { enabled: false, minutes: vec![5, 0] });
        assert!(c.due_alarms(t0 + 99_000).is_empty()); // switched off
        c.set_alarm(111004, AlarmConfig { enabled: true, minutes: vec![5, 0] });
        // App starts an hour after the respawn: nothing fires.
        assert!(c.due_alarms(t0 + 3_700_000).is_empty());
    }

    #[test]
    fn a_boss_that_comes_up_keeps_the_time_its_timer_ran_out() {
        let mut c = core();
        let t0 = 1_000_000_000_000u64;
        c.apply_list(&list(111007, t0 + 60_000), t0);
        let up = BossList { list_id: 0x0456, entries: vec![ListEntry { id: 111007, spawned: true, respawn_ms: None }] };
        c.apply_list(&up, t0 + 300_000); // seen up 4 min after the timer ran out
        c.apply_list(&up, t0 + 400_000); // still up on the next list
        let s = c.snapshot();
        let b = s.bosses.iter().find(|b| b.id == 111007).unwrap();
        assert!(b.spawned);
        assert_eq!(b.spawned_at_local_ms, Some(t0 as i64 + 60_000));

        // Killed and listed with a new timer: the old spawn time is gone.
        c.apply_list(&list(111007, t0 + 4_000_000), t0 + 500_000);
        let s = c.snapshot();
        assert_eq!(s.bosses.iter().find(|b| b.id == 111007).unwrap().spawned_at_local_ms, None);
    }

    #[test]
    fn a_boss_nobody_configured_alarms_15_minutes_ahead() {
        let mut c = core();
        let t0 = 1_000_000_000_000u64;
        c.apply_list(&list(111008, t0 + 20 * 60_000), t0); // respawns in 20 min
        assert!(c.due_alarms(t0 + 4 * 60_000).is_empty()); // 16 min left
        let a = c.due_alarms(t0 + 5 * 60_000 + 1_000); // 14m59s left
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].minutes, 15);
        assert!(c.snapshot().bosses.iter().find(|b| b.id == 111008).unwrap().alarm.enabled);
    }

    #[test]
    fn an_event_alarms_before_its_start_and_again_the_next_hour() {
        let mut c = core();
        c.set_alarm(90002, AlarmConfig { enabled: true, minutes: vec![15, 0] }); // Dimensional Invasion, :30
        // 2026-10-12 12:00 UTC is a Monday. 15 minutes before :30 is :15.
        let noon = 1_791_806_400_000u64;
        assert!(c.due_alarms(noon + 14 * 60_000).is_empty());
        let a = c.due_alarms(noon + 15 * 60_000 + 1_000);
        assert_eq!(a.len(), 1);
        assert_eq!((a[0].kind, a[0].minutes), ("event", 15));
        let a = c.due_alarms(noon + 30 * 60_000 + 1_000);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].minutes, 0);
        // The next hour arms it again.
        assert_eq!(c.due_alarms(noon + 75 * 60_000 + 1_000).len(), 1);
    }

    #[test]
    fn server_clock_offset_shifts_the_deadline() {
        let mut c = core();
        c.note_server_time(1_000_000_010_000, 1_000_000_000_000); // server 10 s ahead
        c.apply_list(&list(111005, 1_000_000_070_000), 1_000_000_000_000);
        let s = c.snapshot();
        let b = s.bosses.iter().find(|b| b.id == 111005).unwrap();
        assert_eq!(b.respawn_local_ms, Some(1_000_000_060_000));
    }

    #[test]
    fn timers_alarms_and_clock_offset_survive_a_restart() {
        let path = std::env::temp_dir().join(format!("boss-alarm-restart-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let t0 = 1_000_000_000_000u64;
        {
            let mut c = Core::load(path.clone());
            c.set_alarm(111006, AlarmConfig { enabled: true, minutes: vec![5, 0] });
            c.note_server_time(t0 + 10_000, t0); // server 10 s ahead
            c.apply_list(&list(111006, t0 + 3_600_000), t0);
        } // app closed

        // Two hours later, before the game connection is seen again.
        let c = Core::load(path.clone());
        let s = c.snapshot();
        let b = s.bosses.iter().find(|b| b.id == 111006).unwrap();
        assert!(b.alarm.enabled);
        assert_eq!(b.respawn_local_ms, Some(t0 as i64 + 3_590_000)); // offset kept
        assert!(!b.spawned);
        assert_eq!(b.seen_local_ms, Some(t0));
        let _ = std::fs::remove_file(&path);
    }
}
