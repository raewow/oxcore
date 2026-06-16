//! Death flow constants and helper functions
//!
//! Contains timing constants, distance calculations, and utility functions
//! used throughout the death and resurrection system.

use super::state::{DeathState, DeathSystemState};

/// Constants
pub const CORPSE_REPOP_TIME_MS: u32 = 360_000; // 6 minutes
pub const CORPSE_RECLAIM_RADIUS: f32 = 39.0; // ~40 yards
pub const CORPSE_RECLAIM_DELAY_NORMAL: u32 = 30; // 30 seconds (PvE death)
pub const CORPSE_RECLAIM_DELAY_PVP: u32 = 120; // 2 minutes (PvP death)
pub const BG_AUTO_RELEASE_MS: u32 = 30_000; // 30 seconds in BG
pub const GHOST_SPEED_MULTIPLIER: f32 = 1.50; // 150% in open world
pub const GHOST_SPEED_MULTIPLIER_BG: f32 = 1.00; // 100% in battlegrounds
pub const CORPSE_RES_HEALTH_PCT: f32 = 0.50; // 50% health on corpse run
pub const CORPSE_RES_MANA_PCT: f32 = 0.50; // 50% mana on corpse run

/// Player flags
pub const PLAYER_FLAGS_GHOST: u32 = 0x0000_0010;

/// Unit flags
pub const UNIT_FLAG_DISABLE_MOVE: u32 = 0x0000_0004;

/// Spell IDs
pub const SPELL_AURA_GHOST: u32 = 8326; // Ghost invisibility aura
pub const SPELL_WISP_FORM: u32 = 20584; // Night Elf ghost speed bonus
pub const SPELL_RESURRECTION_SICKNESS: u32 = 15007; // Applied at spirit healer

/// Race IDs
pub const RACE_NIGHTELF: u8 = 4;

/// Determine the corpse reclaim delay based on death type.
/// PvP deaths impose a longer delay to prevent rapid re-engagement.
pub fn get_corpse_reclaim_delay(is_pvp_death: bool) -> u32 {
    if is_pvp_death {
        CORPSE_RECLAIM_DELAY_PVP
    } else {
        CORPSE_RECLAIM_DELAY_NORMAL
    }
}

/// vmangos reclaim-delay escalation: if the player has died recently, each
/// consecutive death adds to the corpse reclaim wait.
///
/// Ladder, with `death_expire_time` = unix seconds when the "death streak"
/// should reset (updated to `now + 30min` on each fresh death):
///   death_expire_time - now > 25 min  →  30s  (0-5 min since first recent death)
///   death_expire_time - now > 20 min  →  60s  (5-10 min)
///   otherwise                         →  120s (10-15 min)
///
/// If `death_expire_time == 0` the player has no recent deaths; return 30s.
pub fn compute_reclaim_delay(death_expire_time: u64, now: u64, is_pvp_death: bool) -> u32 {
    // PvP always 120s, regardless of recent death count (matches vmangos).
    if is_pvp_death {
        return CORPSE_RECLAIM_DELAY_PVP;
    }
    if death_expire_time <= now {
        return CORPSE_RECLAIM_DELAY_NORMAL;
    }
    let remaining = death_expire_time - now;
    if remaining > 25 * 60 {
        30
    } else if remaining > 20 * 60 {
        60
    } else {
        120
    }
}

/// Bones expiration: 3 days from creation.
pub const BONES_EXPIRE_SECS: u64 = 3 * 24 * 60 * 60;

/// Step added to `death_expire_time` on each fresh death. Within this window
/// consecutive deaths escalate the reclaim delay.
pub const DEATH_EXPIRE_STEP_SECS: u64 = 30 * 60;

/// Check whether a ghost player is close enough to their corpse to resurrect.
/// The client also enforces this check, but the server must validate independently.
pub fn is_within_corpse_reclaim_range(
    ghost_x: f32,
    ghost_y: f32,
    ghost_z: f32,
    corpse_x: f32,
    corpse_y: f32,
    corpse_z: f32,
) -> bool {
    let dx = ghost_x - corpse_x;
    let dy = ghost_y - corpse_y;
    let dz = ghost_z - corpse_z;
    let dist_sq = dx * dx + dy * dy + dz * dz;
    dist_sq <= CORPSE_RECLAIM_RADIUS * CORPSE_RECLAIM_RADIUS
}

/// Update the death timer each tick. Returns true if the timer has expired
/// (corpse should be auto-converted to bones, or auto-release triggered).
pub fn tick_death_timer(state: &mut DeathSystemState, diff_ms: u32) -> bool {
    if state.death_timer_ms == 0 {
        return false;
    }
    if diff_ms >= state.death_timer_ms {
        state.death_timer_ms = 0;
        true
    } else {
        state.death_timer_ms -= diff_ms;
        false
    }
}

/// Check if the player can release spirit (must be in Corpse state)
pub fn can_release_spirit(state: &DeathSystemState) -> bool {
    matches!(state.death_state, DeathState::Corpse)
}

/// Check if the player can reclaim corpse (must be in Dead state with valid corpse)
pub fn can_reclaim_corpse(state: &DeathSystemState) -> bool {
    state.death_state == DeathState::Dead && state.corpse_guid.is_some()
}

/// Get the release timer based on context (battleground vs open world)
pub fn get_release_timer_ms(in_battleground: bool) -> u32 {
    if in_battleground {
        BG_AUTO_RELEASE_MS
    } else {
        CORPSE_REPOP_TIME_MS
    }
}
