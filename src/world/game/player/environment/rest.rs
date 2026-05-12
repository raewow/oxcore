use super::state::{EnvironmentState, RestType};

/// Rate at which rest XP accumulates: 1/8 of a "bubble" per second while resting.
/// One bubble = 5% of the XP needed for the current level.
/// At maximum accumulation rate, it takes about 8 days offline to fill
/// the full 1.5 levels of rest XP.
pub const REST_RATE_PER_SECOND: f32 = 0.125;

/// PLAYER_FLAGS bit for the resting visual indicator (Zzz icon)
pub const PLAYER_FLAGS_RESTING: u32 = 0x00000020;

/// Update rest bonus accumulation (called from player update loop).
///
/// `diff_ms` is the elapsed time in milliseconds since the last tick.
/// `next_level_xp` is the total XP required for the player's next level.
///
/// The formula:
///   bubble_xp = next_level_xp / 20   (5% of a level)
///   gain = (seconds_elapsed) * REST_RATE_PER_SECOND * bubble_xp
///   max_rest = next_level_xp * 1.5
pub fn update_rest_bonus(state: &mut EnvironmentState, diff_ms: u32, next_level_xp: u32) {
    if state.rest_type == RestType::No {
        return;
    }

    let bubble_xp = next_level_xp as f32 / 20.0; // 5% of level
    let seconds = diff_ms as f32 / 1000.0;
    let gain = seconds * REST_RATE_PER_SECOND * bubble_xp;
    let max_rest = next_level_xp as f32 * 1.5;

    state.rest_bonus = (state.rest_bonus + gain).min(max_rest);
}

/// Apply rest bonus to a kill XP award.
///
/// Returns the bonus XP to add on top of `base_xp`. When rested, the player
/// receives up to 100% bonus (200% total) on kill XP. The rest pool is
/// consumed by the bonus amount.
///
/// Example: Player kills a mob for 100 base XP with 500 rest bonus remaining.
///   bonus = min(100, 500) = 100
///   rest_bonus becomes 400
///   Player receives 100 (base) + 100 (rest) = 200 XP total.
pub fn apply_rest_bonus(state: &mut EnvironmentState, base_xp: u32) -> u32 {
    if state.rest_bonus <= 0.0 {
        return 0;
    }

    let bonus = (base_xp as f32).min(state.rest_bonus) as u32;
    state.rest_bonus -= bonus as f32;
    bonus
}

/// Enter or exit a rest state.
///
/// When entering rest (InTavern or InCity), sets the PLAYER_FLAGS_RESTING bit,
/// records the area trigger ID and entry timestamp.
/// When exiting rest (No), clears the flag.
pub fn set_rest_type(
    state: &mut EnvironmentState,
    rest_type: RestType,
    area_trigger_id: u32,
    player_flags: &mut u32,
) {
    state.rest_type = rest_type;

    if rest_type == RestType::No {
        *player_flags &= !PLAYER_FLAGS_RESTING;
    } else {
        *player_flags |= PLAYER_FLAGS_RESTING;
        state.inn_trigger_id = area_trigger_id;
        state.time_inn_enter = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

/// Calculate rest XP accumulated while the player was offline.
///
/// `time_offline_secs` - seconds since the player last logged out.
/// `next_level_xp` - XP required for the player's next level at login.
/// `offline_rate_multiplier` - server config rate (default 1.0, often set to 4.0).
/// `was_resting` - whether the player logged out in a rest area.
///
/// Players who log out outside a rest area accumulate rest at 1/4 the normal
/// rate (vanilla behavior). The server config multiplier applies on top.
///
/// Returns the rest XP to add to the player's existing rest_bonus (capped at 1.5 levels).
pub fn calculate_offline_rest(
    time_offline_secs: u64,
    next_level_xp: u32,
    offline_rate_multiplier: f32,
    was_resting: bool,
) -> f32 {
    let bubble_xp = next_level_xp as f32 / 20.0;

    // Offline in rest area: full rate. Offline elsewhere: 1/4 rate.
    let location_factor = if was_resting { 1.0 } else { 0.25 };

    let gain = time_offline_secs as f32
        * REST_RATE_PER_SECOND
        * bubble_xp
        * offline_rate_multiplier
        * location_factor;

    let max_rest = next_level_xp as f32 * 1.5;
    gain.min(max_rest)
}

/// Called during player login to restore rest state.
///
/// 1. Load rest_bonus and rest_type from the characters table.
/// 2. Calculate offline rest gain since last logout.
/// 3. Add offline gain to existing rest_bonus (capped).
/// 4. Send initial rest state to the client.
pub fn on_player_login(
    state: &mut EnvironmentState,
    saved_rest_bonus: f32,
    saved_rest_type: RestType,
    logout_timestamp: u64,
    next_level_xp: u32,
    offline_rate_multiplier: f32,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let time_offline = now.saturating_sub(logout_timestamp);
    let was_resting = saved_rest_type != RestType::No;

    let offline_gain = calculate_offline_rest(
        time_offline,
        next_level_xp,
        offline_rate_multiplier,
        was_resting,
    );

    let max_rest = next_level_xp as f32 * 1.5;
    state.rest_bonus = (saved_rest_bonus + offline_gain).min(max_rest);
    state.rest_type = saved_rest_type;
}
