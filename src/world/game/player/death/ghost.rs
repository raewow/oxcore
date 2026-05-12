//! Ghost form management
//!
//! Handles the transformation to and from ghost form when a player
/// releases their spirit or is resurrected.
use super::flow::*;
use super::state::DeathState;

/// Properties applied during the build_player_repop transition:
///
/// 1. SPELL_AURA_GHOST (spell 8326)
///    - Aura type: SPELL_AURA_GHOST
///    - Makes the player invisible to all living players
///    - Living players cannot target or interact with the ghost
///    - Other ghosts CAN see each other
///    - Hunters with "Track Undead" active can see ghosts on their minimap
///
/// 2. PLAYER_FLAGS_GHOST (0x10)
///    - Server-side flag checked by interaction validators
///    - Prevents interaction with living NPCs (except spirit healers)
///    - Prevents picking up quest items, opening chests, etc.
///    - Prevents casting spells (except self-resurrection abilities)
///
/// 3. Movement speed
///    - Open world: 150% of base run speed
///    - Battlegrounds: 100% (no speed bonus)
///    - Night Elves get an additional speed bonus via Wisp Spirit (spell 20584)
///
/// 4. Water walking
///    - Ghosts can walk on all water surfaces
///    - Implemented via SMSG_MOVE_WATER_WALK packet
///    - Removed on resurrection
///
/// 5. Health
///    - Ghost health is set to 1 (not 0 - ghosts are technically "alive"
///      from the movement system's perspective, but flagged as dead by
///      PLAYER_FLAGS_GHOST for combat/interaction purposes)

/// Build the ghost form for a player who has just released spirit.
///
/// This is called from handle_repop_request after the player clicks
/// "Release Spirit" on the death screen.
///
/// Returns a list of spell IDs that must be cast on the player after
/// the function returns (we cannot cast while holding a mutable borrow
/// on the player).
pub fn build_player_repop(
    player_race: u8,
    player_health: &mut u32,
    player_flags: &mut u32,
    death_state: &mut DeathState,
) -> Vec<u32> {
    let mut pending_spells = Vec::new();

    // Set health to 1 (ghosts have 1 health)
    *player_health = 1;

    // Set ghost player flag
    *player_flags |= PLAYER_FLAGS_GHOST;

    // Set death state to Dead (ghost form)
    *death_state = DeathState::Dead;

    // Queue ghost aura
    pending_spells.push(SPELL_AURA_GHOST);

    // Night Elves get Wisp Spirit for faster ghost movement
    if player_race == RACE_NIGHTELF {
        pending_spells.push(SPELL_WISP_FORM);
    }

    pending_spells
}

/// Remove ghost form during resurrection.
///
/// Called as part of the JustAlived transition. Reverses all
/// effects applied by build_player_repop.
pub fn remove_ghost_form(player_race: u8, player_flags: &mut u32) -> Vec<u32> {
    let mut auras_to_remove = Vec::new();

    // Clear ghost flag
    *player_flags &= !PLAYER_FLAGS_GHOST;

    // Remove ghost aura
    auras_to_remove.push(SPELL_AURA_GHOST);

    // Remove wisp form if Night Elf
    if player_race == RACE_NIGHTELF {
        auras_to_remove.push(SPELL_WISP_FORM);
    }

    auras_to_remove
}

/// Get the ghost speed multiplier based on context
pub fn get_ghost_speed_multiplier(in_battleground: bool) -> f32 {
    if in_battleground {
        GHOST_SPEED_MULTIPLIER_BG
    } else {
        GHOST_SPEED_MULTIPLIER
    }
}
