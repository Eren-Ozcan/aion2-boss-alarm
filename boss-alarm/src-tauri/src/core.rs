//! Timers, alarm settings and the alarm scheduler.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::bosses::{self, BOSSES};
use crate::parser::BossList;

/// Minutes before the respawn at which to alarm; 0 is the respawn itself.
fn default_minutes() -> Vec<u32> {
    vec![5, 0]
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AlarmConfig {
    pub enabled: bool,
    #[serde(default = "default_minutes")]
    pub minutes: Vec<u32>,
}

impl Default for AlarmConfig {
    fn default() -> Self {
        Self { enabled: false, minutes: default_minutes() }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct StoredTimer {
    respawn_ms: Option<u64>,
    spawned: bool,
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
    pub alarm: AlarmConfig,
}

#[derive(Clone, Serialize)]
pub struct Snapshot {
    pub status: Status,
    pub bosses: Vec<BossView>,
    pub last_list_local_ms: Option<u64>,
}

#[derive(Clone, Serialize)]
pub struct AlarmEvent {
    pub id: u32,
    pub name: &'static str,
    pub minutes: u32,
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
            self.stored.timers.insert(
                e.id,
                StoredTimer { respawn_ms: e.respawn_ms, spawned: e.spawned, seen_local_ms: local_ms },
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
                    alarm: self.stored.alarms.get(&b.id).cloned().unwrap_or_default(),
                }
            })
            .collect();
        Snapshot { status: self.status.clone(), bosses, last_list_local_ms: self.last_list_local_ms }
    }

    /// Alarms that are due at `local_ms`. Each (boss, respawn time, minutes)
    /// fires once; a new respawn time (the boss was killed again) re-arms it.
    pub fn due_alarms(&mut self, local_ms: u64) -> Vec<AlarmEvent> {
        let mut out = Vec::new();
        for b in BOSSES {
            let Some(cfg) = self.stored.alarms.get(&b.id).filter(|c| c.enabled) else { continue };
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
                        out.push(AlarmEvent { id: b.id, name: b.name, minutes: m });
                    }
                }
            }
        }
        // Forget keys of timers that no longer exist.
        if self.fired.len() > 4096 {
            self.fired.clear();
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
        assert!(c.due_alarms(t0 + 99_000).is_empty()); // not enabled
        c.set_alarm(111004, AlarmConfig { enabled: true, minutes: vec![5, 0] });
        // App starts an hour after the respawn: nothing fires.
        assert!(c.due_alarms(t0 + 3_700_000).is_empty());
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
