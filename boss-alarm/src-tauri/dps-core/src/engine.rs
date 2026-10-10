//! Glue that turns reassembled game-stream bytes into live DPS numbers.
//!
//! Written for this project: it wires the A2Tools-derived parser (see
//! `NOTICE.md`) to one entry point so the app only has to feed bytes and read
//! a snapshot.

use std::collections::{HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;

use crate::capture::stream_assembler::StreamAssembler;
use crate::capture::stream_processor::StreamProcessor;
use crate::combat::data_storage::DataStorage;
use crate::combat::dps_calculator::DpsCalculator;
use crate::combat::ping_tracker::PingTracker;
use crate::entity::details_context::TargetDetailsResponse;
use crate::entity::dps_data::DpsData;
use crate::entity::fight_record::FightRecord;
use crate::history::fight_history::FightHistoryManager;
use crate::i18n::lookup::{NpcLookup, SkillLookup};

const SKILLS_EN: &str = include_str!("data/i18n/skills/en.json");
const NPCS_EN: &str = include_str!("data/i18n/npcs/en.json");
const DOT_SKILL_IDS: &str = include_str!("data/dot_skill_ids.json");

/// The remembered name and whether the player typed it (as opposed to the game
/// having said it).
fn read_name(path: &std::path::Path) -> Option<(String, bool)> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    let name = v.get("name")?.as_str()?.trim().to_string();
    let manual = v.get("manual").and_then(|m| m.as_bool()).unwrap_or(false);
    (!name.is_empty() && name.chars().count() <= 24).then_some((name, manual))
}

/// The game talks over several TCP connections at once (world and instance
/// servers, all on one port), each its own byte stream with its own framing.
/// Mixing them in one parser garbles both, so each gets its own assembler and
/// processor, all feeding the same combat data.
struct Stream {
    assembler: StreamAssembler,
    processor: StreamProcessor,
    used: u64,
}

/// More connections than this at once is not the game; drop the least recent.
const MAX_STREAMS: usize = 12;

pub struct DpsEngine {
    streams: HashMap<u64, Stream>,
    clock: u64,
    skills: Arc<SkillLookup>,
    npcs: Arc<NpcLookup>,
    dot_ids: HashSet<i32>,
    calculator: DpsCalculator,
    history: Arc<FightHistoryManager>,
    ping: Arc<PingTracker>,
    storage: Arc<DataStorage>,
    /// Where the remembered character name lives (`character.json`).
    identity_file: PathBuf,
    remembered: Option<String>,
    /// The name was typed by the player, so it is applied to your row even when
    /// the game only gave your id indirectly.
    manual_name: bool,
    /// Count every boss fight in range, not only ones you or your party are in.
    every_fight: bool,
}

impl DpsEngine {
    /// `data_dir` is where finished fights are kept (under `history/`).
    pub fn new(data_dir: PathBuf) -> Self {
        let skills = SkillLookup::new();
        skills.load_from_json(SKILLS_EN);
        let npcs = NpcLookup::new();
        npcs.load_from_json(NPCS_EN);
        let (skills, npcs) = (Arc::new(skills), Arc::new(npcs));

        let storage = Arc::new(DataStorage::new());
        // The game names you only in a few packets (login, zone change), so an
        // app started mid-session would show you as a bare id. Start from the
        // name remembered from last time; the game's own record replaces it.
        let identity_file = data_dir.join("character.json");
        let (remembered, manual) = match read_name(&identity_file) {
            Some((name, manual)) => (Some(name), manual),
            None => (None, false),
        };
        if let Some(name) = &remembered {
            storage.set_local_character_name(Some(name.clone()));
        }
        let ping = Arc::new(PingTracker::with_perf_clock(crate::perf_clock::perf_clock()));
        let dot_ids: Vec<i32> = serde_json::from_str(DOT_SKILL_IDS).unwrap_or_default();
        let calculator = DpsCalculator::new(storage.clone(), skills.clone(), npcs.clone(), ping.clone());
        Self {
            streams: HashMap::new(),
            clock: 0,
            skills,
            npcs,
            dot_ids: dot_ids.into_iter().collect(),
            calculator,
            history: Arc::new(FightHistoryManager::new(data_dir)),
            ping,
            storage,
            identity_file,
            remembered,
            manual_name: manual,
            every_fight: false,
        }
    }

    /// Feed in-order bytes of one game connection (server to client). `flow`
    /// names the connection; bytes of different flows are never mixed.
    ///
    /// A packet the parser chokes on must not take the capture thread (and the
    /// boss alarms riding on it) down: a panic drops that connection's buffered
    /// bytes and parsing resumes with the next chunk.
    pub fn feed(&mut self, flow: u64, bytes: &[u8]) -> bool {
        self.clock += 1;
        if !self.streams.contains_key(&flow) {
            if self.streams.len() >= MAX_STREAMS {
                if let Some(oldest) = self.streams.iter().min_by_key(|(_, s)| s.used).map(|(k, _)| *k) {
                    self.streams.remove(&oldest);
                }
            }
            let mut processor = StreamProcessor::new(self.storage.clone(), self.skills.clone(), self.npcs.clone());
            processor.set_dot_skill_ids(self.dot_ids.clone());
            self.streams.insert(flow, Stream { assembler: StreamAssembler::new(), processor, used: 0 });
        }
        let stream = self.streams.get_mut(&flow).expect("just inserted");
        stream.used = self.clock;
        let parsed = catch_unwind(AssertUnwindSafe(|| stream.assembler.process_chunk(bytes, &mut stream.processor)));
        parsed.unwrap_or_else(|_| {
            tracing::error!("packet parser panicked; dropping buffered bytes");
            stream.assembler = StreamAssembler::new();
            false
        })
    }

    pub fn snapshot(&mut self) -> DpsData {
        self.remember_identity();
        self.calculator.get_dps()
    }

    /// Your character name, as typed or last learnt.
    pub fn character_name(&self) -> Option<String> {
        self.storage.local_character_name().filter(|n| !n.trim().is_empty())
    }

    /// A name typed by the player. Taken as given; the game's next self record
    /// replaces it if it was wrong.
    pub fn set_character_name(&mut self, name: &str) {
        let name = name.trim();
        if name.chars().count() > 24 {
            return;
        }
        self.storage.set_local_character_name(if name.is_empty() { None } else { Some(name.to_string()) });
        if let Some(id) = self.storage.local_player_id().filter(|_| !name.is_empty()) {
            self.storage.set_permanent_nickname(id as i32, name);
        }
        self.manual_name = !name.is_empty();
        self.write_name(name);
    }

    /// Once the game has said who you are, keep that name for the next start.
    fn remember_identity(&mut self) {
        if !self.storage.local_identity_from_game() {
            // The game has not named you yet: show the remembered or typed name on your
            // row. This holds also when the id was only worked out from the fight (an app
            // started mid-session never sees the self record): a bare #id there made the
            // player look for themselves in vain. A wrong name, after switching characters
            // between sessions, is fixed by typing the name in the DPS tab, and the game's
            // own record replaces it at the next login or zone change.
            if let (Some(id), Some(name)) = (self.storage.local_player_id(), self.character_name()) {
                let bare = self.storage.get_nickname(id as i32).is_none_or(|n| n.chars().all(|c| c.is_ascii_digit()));
                if bare {
                    self.storage.set_permanent_nickname(id as i32, &name);
                }
            }
            return;
        }
        let Some(id) = self.storage.local_player_id() else { return };
        let Some(name) = self.storage.get_nickname(id as i32).filter(|n| !n.trim().is_empty() && !n.chars().all(|c| c.is_ascii_digit())) else { return };
        if self.remembered.as_deref() != Some(name.as_str()) {
            self.manual_name = false;
            self.write_name(&name);
        }
    }

    fn write_name(&mut self, name: &str) {
        self.remembered = Some(name.to_string());
        if let Ok(json) = serde_json::to_string(&serde_json::json!({ "name": name, "manual": self.manual_name })) {
            let _ = crate::atomic_file::write(&self.identity_file, json.as_bytes());
        }
    }

    /// Forget the current fight's damage and start counting again.
    pub fn reset(&mut self) {
        self.calculator.restart_target_selection(true);
    }

    /// The ping tracker, to be fed every segment of the game connection (both
    /// directions) without going through the engine's lock.
    pub fn ping_tracker(&self) -> Arc<PingTracker> {
        self.ping.clone()
    }

    pub fn history(&self) -> Arc<FightHistoryManager> {
        self.history.clone()
    }

    /// Boss fights that finished since the last call. Save them outside the
    /// engine's lock so disk work never stalls packet parsing.
    ///
    /// History keeps only fights you were part of, whatever the live view
    /// shows: a boss strangers fought in range should not fill it.
    pub fn finished_fights(&mut self) -> Vec<FightRecord> {
        self.calculator.set_every_fight(false);
        let fights = self.calculator.snapshot_boss_fights();
        self.calculator.set_every_fight(self.every_fight);
        fights
    }

    /// Show a boss fight whenever one is in range, even one you are not in
    /// (the meter also reads that way before it knows who you are). Off, only
    /// fights of you or your party count, so strangers' fights stay off it.
    pub fn debug_targets(&self) -> Vec<String> {
        self.calculator.debug_targets()
    }

    pub fn set_every_fight(&mut self, on: bool) {
        self.every_fight = on;
        self.calculator.set_every_fight(on);
    }

    /// Debug view of who the meter thinks you are: local actor id, whether the
    /// id came from party counts only, whether the game stated it, your name.
    pub fn identity_state(&self) -> (Option<i64>, bool, bool, Option<String>) {
        let id = self.storage.local_player_id();
        (
            id,
            self.storage.local_id_from_scope(),
            self.storage.local_identity_from_game(),
            id.and_then(|i| self.storage.get_nickname(i as i32)),
        )
    }

    /// Every boss fight seen, whoever fought it and however recent. For
    /// checking a replayed recording against another meter, not for saving.
    pub fn all_boss_fights(&mut self) -> Vec<FightRecord> {
        self.calculator.snapshot_boss_fights_force()
    }

    /// One of `bossTargets`, `lastHitByMe`, `allTargets`, `trainTargets`.
    pub fn set_target_mode(&mut self, id: &str) {
        self.calculator.set_target_selection_mode(id);
    }

    /// Skill rows of every player on one target (for the timeline chart).
    pub fn details_all(&self, target_id: i32) -> TargetDetailsResponse {
        self.calculator.get_target_details(target_id, None)
    }

    /// Skill table for one player on one target.
    pub fn details(&self, target_id: i32, actor_id: i32) -> TargetDetailsResponse {
        self.calculator.get_target_details(target_id, Some(&[actor_id]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Garbage and truncated input must never panic the parser. (Deterministic
    /// pseudo-random bytes, no extra dependency.)
    #[test]
    fn random_bytes_do_not_panic_the_parser() {
        let mut engine = DpsEngine::new(std::env::temp_dir().join("dps-core-fuzz"));
        let mut x: u64 = 0x2545F4914F6CDD1D;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for round in 0..400 {
            let len = (next() % 4000) as usize;
            let mut buf: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            // Make it look like framing now and then: plausible length prefixes.
            if round % 3 == 0 && buf.len() > 8 {
                buf[0] = 0x20 + (next() % 0x60) as u8;
                buf[1] = 0x04;
                buf[2] = 0x8d;
            }
            engine.feed(1 + (round % 3) as u64, &buf);
            let _ = engine.snapshot();
        }
    }
}
