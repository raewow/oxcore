//! Contributor tracking — records which players recently dealt damage to
//! this unit, used by the honor system to distribute PvP kill credit.

use crate::shared::protocol::ObjectGuid;

/// Keep contributors that dealt damage within the last 60 seconds.
pub const CONTRIBUTOR_WINDOW_SECS: u64 = 60;

/// Max number of distinct attackers we remember per victim. vmangos keeps an
/// effectively-unbounded list; 20 is far more than typical practical combat.
const MAX_CONTRIBUTORS: usize = 20;

/// A single attacker's damage contribution.
#[derive(Debug, Clone, Copy)]
pub struct Contributor {
    pub guid: ObjectGuid,
    /// Unix seconds of the most recent damage event from this attacker.
    pub last_damage_time: u64,
    /// Total damage dealt across the current window.
    pub total_damage: u64,
}

impl Contributor {
    fn new(guid: ObjectGuid, now: u64, damage: u32) -> Self {
        Self {
            guid,
            last_damage_time: now,
            total_damage: damage as u64,
        }
    }
}

/// Fixed-size ring of recent damage contributors. Purged lazily on each
/// `record_damage` call.
#[derive(Debug, Default, Clone)]
pub struct ContributorTracker {
    entries: Vec<Contributor>,
}

impl ContributorTracker {
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(MAX_CONTRIBUTORS),
        }
    }

    /// Record a damage hit from `attacker` at time `now`.
    pub fn record_damage(&mut self, attacker: ObjectGuid, now: u64, damage: u32) {
        self.purge_expired(now);

        if let Some(c) = self.entries.iter_mut().find(|c| c.guid == attacker) {
            c.last_damage_time = now;
            c.total_damage = c.total_damage.saturating_add(damage as u64);
            return;
        }

        if self.entries.len() >= MAX_CONTRIBUTORS {
            // Drop the entry with the smallest total_damage (least
            // consequential contributor) to make room.
            if let Some((idx, _)) = self
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, c)| c.total_damage)
            {
                self.entries.swap_remove(idx);
            }
        }
        self.entries.push(Contributor::new(attacker, now, damage));
    }

    /// Remove entries older than `CONTRIBUTOR_WINDOW_SECS`.
    pub fn purge_expired(&mut self, now: u64) {
        self.entries
            .retain(|c| now.saturating_sub(c.last_damage_time) <= CONTRIBUTOR_WINDOW_SECS);
    }

    /// Drain a snapshot of current contributors (after purging) and reset.
    /// Called on the victim's death to distribute honor.
    pub fn take_contributors(&mut self, now: u64) -> Vec<Contributor> {
        self.purge_expired(now);
        std::mem::take(&mut self.entries)
    }

    /// Read-only snapshot (does not reset state).
    pub fn contributors(&self) -> &[Contributor] {
        &self.entries
    }

    /// Wipe the tracker (used on resurrect, zoning out, etc).
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guid(n: u32) -> ObjectGuid {
        ObjectGuid::new_player(n)
    }

    #[test]
    fn records_new_attacker() {
        let mut t = ContributorTracker::new();
        t.record_damage(guid(1), 100, 50);
        assert_eq!(t.contributors().len(), 1);
        assert_eq!(t.contributors()[0].total_damage, 50);
    }

    #[test]
    fn accumulates_repeat_attacker() {
        let mut t = ContributorTracker::new();
        t.record_damage(guid(1), 100, 50);
        t.record_damage(guid(1), 110, 30);
        assert_eq!(t.contributors().len(), 1);
        assert_eq!(t.contributors()[0].total_damage, 80);
    }

    #[test]
    fn purges_after_window() {
        let mut t = ContributorTracker::new();
        t.record_damage(guid(1), 100, 50);
        // Record from attacker 2 at t=200 — attacker 1 is now > 60s stale
        // and should be purged.
        t.record_damage(guid(2), 200, 10);
        assert_eq!(t.contributors().len(), 1);
        assert_eq!(t.contributors()[0].guid, guid(2));
    }
}
