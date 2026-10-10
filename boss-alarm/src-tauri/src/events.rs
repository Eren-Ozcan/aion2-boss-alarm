//! Recurring in-game events with a fixed schedule. Unlike the bosses, the game
//! does not send their times to us, so they are computed from the schedule the
//! community timer sites agree on (hourly :00 and :30, siege three nights a week).
//! Times are in the game's server time; the server's offset from UTC is a setting.

pub const HOUR_MS: i64 = 3_600_000;
pub const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 24 * HOUR_MS;

/// The European server clock, UTC+2: the siege that opens at 21:00 server time
/// started at 22:00 in Turkey (UTC+3). It is a setting because the game publishes
/// no schedule for the global servers and other regions differ.
pub const DEFAULT_SERVER_OFFSET_H: i32 = 2;

#[derive(Clone, Copy)]
pub enum Rule {
    /// Every hour at this minute.
    Hourly { minute: i64 },
    /// On these weekdays (0 = Sunday) at this server time.
    Weekly { days: &'static [i64], hour: i64, minute: i64 },
    /// Every `hours` hours, starting at `first_hour` server time each day.
    Every { hours: i64, first_hour: i64 },
}

pub struct EventInfo {
    pub id: u32,
    pub name: &'static str,
    pub rule: Rule,
}

pub const EVENTS: &[EventInfo] = &[
    EventInfo { id: 90001, name: "Shugo Festival", rule: Rule::Hourly { minute: 0 } },
    EventInfo { id: 90002, name: "Dimensional Invasion", rule: Rule::Hourly { minute: 30 } },
    EventInfo { id: 90003, name: "Abyss Siege", rule: Rule::Weekly { days: &[1, 4, 6], hour: 21, minute: 0 } },
    EventInfo { id: 90004, name: "Spacetime Rift", rule: Rule::Every { hours: 3, first_hour: 2 } },
];

impl Rule {
    /// Time between two occurrences, for a progress bar.
    pub fn period_ms(&self) -> i64 {
        match self {
            Rule::Hourly { .. } => HOUR_MS,
            Rule::Weekly { .. } => 3 * DAY_MS,
            Rule::Every { hours, .. } => hours * HOUR_MS,
        }
    }

    /// The first occurrence at or after `from_ms` (unix ms, UTC).
    pub fn next_at_or_after(&self, from_ms: i64, server_offset_h: i32) -> i64 {
        match *self {
            Rule::Hourly { minute } => {
                let base = from_ms.div_euclid(HOUR_MS) * HOUR_MS + minute * MINUTE_MS;
                if base >= from_ms { base } else { base + HOUR_MS }
            }
            Rule::Every { hours, first_hour } => {
                let off = i64::from(server_offset_h) * HOUR_MS;
                let period = hours * HOUR_MS;
                let anchor = first_hour * HOUR_MS;
                let server_from = from_ms + off;
                let n = (server_from - anchor + period - 1).div_euclid(period);
                n * period + anchor - off
            }
            Rule::Weekly { days, hour, minute } => {
                let off = i64::from(server_offset_h) * HOUR_MS;
                let server_from = from_ms + off;
                let day0 = server_from.div_euclid(DAY_MS);
                for d in 0..=8 {
                    let day = day0 + d;
                    // 1970-01-01 was a Thursday (4).
                    let dow = (day + 4).rem_euclid(7);
                    if !days.contains(&dow) {
                        continue;
                    }
                    let at = day * DAY_MS + hour * HOUR_MS + minute * MINUTE_MS;
                    if at >= server_from {
                        return at - off;
                    }
                }
                from_ms + 7 * DAY_MS // unreachable for a non-empty weekday list
            }
        }
    }

    /// A short label: "Every hour :30", "Mon Thu Sat 21:00".
    pub fn label(&self) -> String {
        match *self {
            Rule::Hourly { minute } => format!(":{minute:02}"),
            Rule::Weekly { days, hour, minute } => {
                const NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                let d: Vec<&str> = days.iter().map(|&d| NAMES[d as usize]).collect();
                format!("{} {hour:02}:{minute:02}", d.join(" "))
            }
            Rule::Every { hours, .. } => format!("{hours}h"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-10-12 is a Monday. 12:00 UTC = 21:00 server time at UTC+9.
    const MON_NOON_UTC: i64 = 1_791_806_400_000;

    #[test]
    fn hourly_events_land_on_their_minute() {
        let r = Rule::Hourly { minute: 30 };
        let t = MON_NOON_UTC + 10 * MINUTE_MS;
        assert_eq!(r.next_at_or_after(t, 9), MON_NOON_UTC + 30 * MINUTE_MS);
        assert_eq!(r.next_at_or_after(MON_NOON_UTC + 30 * MINUTE_MS, 9), MON_NOON_UTC + 30 * MINUTE_MS);
        assert_eq!(r.next_at_or_after(MON_NOON_UTC + 31 * MINUTE_MS, 9), MON_NOON_UTC + 90 * MINUTE_MS);
    }

    #[test]
    fn the_siege_is_monday_thursday_saturday_in_server_time() {
        let r = Rule::Weekly { days: &[1, 4, 6], hour: 21, minute: 0 };
        // Monday 12:00 UTC is Monday 21:00 server time (UTC+9): that is the siege.
        assert_eq!(r.next_at_or_after(MON_NOON_UTC, 9), MON_NOON_UTC);
        // A minute later the next one is Thursday.
        assert_eq!(r.next_at_or_after(MON_NOON_UTC + MINUTE_MS, 9), MON_NOON_UTC + 3 * DAY_MS);
        // After Saturday's, the next is the Monday after.
        let sat = MON_NOON_UTC + 5 * DAY_MS;
        assert_eq!(r.next_at_or_after(sat + MINUTE_MS, 9), MON_NOON_UTC + 7 * DAY_MS);
    }

    #[test]
    fn a_different_server_offset_moves_the_siege() {
        let r = Rule::Weekly { days: &[1, 4, 6], hour: 21, minute: 0 };
        // At UTC+2 the siege is 21:00 UTC+2 = 19:00 UTC, seven hours after the UTC+9 one.
        assert_eq!(r.next_at_or_after(MON_NOON_UTC - HOUR_MS, 2), MON_NOON_UTC + 7 * HOUR_MS);
    }

    #[test]
    fn the_rift_comes_every_three_hours_from_02_server_time() {
        let r = Rule::Every { hours: 3, first_hour: 2 };
        // Server UTC+2: Monday 12:00 UTC is 14:00 server, which is a rift (02, 05, 08, 11, 14, 17 ...).
        assert_eq!(r.next_at_or_after(MON_NOON_UTC, 2), MON_NOON_UTC);
        // A moment later the next is 17:00 server = 15:00 UTC.
        assert_eq!(r.next_at_or_after(MON_NOON_UTC + 1, 2), MON_NOON_UTC + 3 * HOUR_MS);
        // In Turkey (UTC+3) the rifts fall on hours divisible by three: 15:00 now, then 18:00, 21:00, 00:00.
        let turkey_hour = (MON_NOON_UTC / HOUR_MS + 3).rem_euclid(24);
        assert_eq!(turkey_hour % 3, 0);
    }

    #[test]
    fn labels_read_well() {
        assert_eq!(Rule::Hourly { minute: 0 }.label(), ":00");
        assert_eq!(Rule::Weekly { days: &[1, 4, 6], hour: 21, minute: 0 }.label(), "Mon Thu Sat 21:00");
    }
}
