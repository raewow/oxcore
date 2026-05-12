//! Resurrection sickness calculation
//!
//! Resurrection sickness is the penalty for using a spirit healer instead
//! of running back to your corpse. It is a debuff that severely reduces
//! the player's combat effectiveness.

/// Resurrection sickness spell ID.
/// Effect: -75% to all stats (Strength, Agility, Stamina, Intellect, Spirit)
/// Effect: -75% to all damage dealt
/// Duration: level-dependent (see get_resurrection_sickness_duration)
pub const SPELL_RESURRECTION_SICKNESS: u32 = 15007;

/// Level below which resurrection sickness is NOT applied.
/// Configurable in worldserver.conf as DeathSicknessLevel (default 10).
pub const DEATH_SICKNESS_LEVEL: u8 = 10;

/// Calculate the duration of resurrection sickness in seconds.
///
/// Rules (vanilla 1.12):
///   - Level 1-10:  No sickness at all (0 seconds)
///   - Level 11-19: (level - 10) minutes (1 to 9 minutes)
///   - Level 20-60: 10 minutes (600 seconds)
///
/// The level threshold (10) is configurable via DeathSicknessLevel in
/// worldserver.conf. The default matches Blizzard's original value.
pub fn get_resurrection_sickness_duration(level: u8) -> u32 {
    if level < DEATH_SICKNESS_LEVEL {
        0
    } else if level < 20 {
        // (level - 10 + 1) minutes
        // Level 11 = 1 min, level 12 = 2 min, ... level 19 = 9 min
        ((level - DEATH_SICKNESS_LEVEL + 1) as u32) * 60
    } else {
        600 // 10 minutes
    }
}

/// Get the race-specific resurrection sickness spell ID.
///
/// In vanilla WoW, all races use spell 15007. However, the ChrRaces.dbc
/// has a res_sickness_spell_id field per race that we check first for
/// correctness. Falls back to 15007 if the DBC entry is missing or zero.
pub fn get_resurrection_sickness_spell_id(
    race: u8,
    dbc_mgr: Option<&crate::world::dbc::DbcManager>,
) -> u32 {
    if let Some(dbc_mgr) = dbc_mgr {
        if let Some(race_entry) = dbc_mgr.get_chr_race(race as u32) {
            if race_entry.res_sickness_spell_id != 0 {
                return race_entry.res_sickness_spell_id;
            }
        }
    }
    SPELL_RESURRECTION_SICKNESS
}

/// Apply resurrection sickness to a player after spirit healer resurrection.
///
/// This function determines the correct spell ID and duration, then returns
/// the spell ID and duration so the caller can apply the aura through the
/// spell system.
///
/// Returns (spell_id, duration_seconds). Duration is 0 if the player is
/// below the sickness level threshold.
pub fn compute_resurrection_sickness(
    level: u8,
    race: u8,
    dbc_mgr: Option<&crate::world::dbc::DbcManager>,
) -> (u32, u32) {
    let duration = get_resurrection_sickness_duration(level);
    if duration == 0 {
        return (0, 0);
    }

    let spell_id = get_resurrection_sickness_spell_id(race, dbc_mgr);
    (spell_id, duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sickness_below_threshold() {
        for level in 1..=9 {
            assert_eq!(get_resurrection_sickness_duration(level), 0);
        }
    }

    #[test]
    fn sickness_transition_levels() {
        assert_eq!(get_resurrection_sickness_duration(10), 60); // 1 min
        assert_eq!(get_resurrection_sickness_duration(11), 120); // 2 min
        assert_eq!(get_resurrection_sickness_duration(15), 360); // 6 min
        assert_eq!(get_resurrection_sickness_duration(19), 600); // 10 min
    }

    #[test]
    fn sickness_max_duration() {
        assert_eq!(get_resurrection_sickness_duration(20), 600);
        assert_eq!(get_resurrection_sickness_duration(40), 600);
        assert_eq!(get_resurrection_sickness_duration(60), 600);
    }
}
