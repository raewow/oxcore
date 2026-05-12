//! Death state definitions
//!
//! Contains the DeathState enum and DeathSystemState struct that tracks
//! all death-related state for a player.

use crate::shared::protocol::ObjectGuid;
use crate::shared::protocol::Position;

/// Death state machine for units (players and creatures).
///
/// Transitions:
///   Alive -> JustDied   (health reaches 0)
///   JustDied -> Corpse   (automatic, one tick)
///   Corpse -> Dead       (player clicks Release Spirit)
///   Dead -> JustAlived   (resurrection accepted)
///   JustAlived -> Alive  (automatic, one tick)
///
/// Creatures skip Corpse/Dead and go JustDied -> Corpse (lootable) -> Dead (despawn).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathState {
    /// Unit is alive and functioning normally.
    Alive,
    /// Transitional: unit just died this tick. Used to trigger one-time death
    /// side effects (aura removal, flag drops, combat stop). Automatically
    /// advances to Corpse on the next update.
    JustDied,
    /// Player's spirit is still at the corpse location. The player sees the
    /// death screen with a "Release Spirit" button. For creatures, this is the
    /// lootable corpse state.
    Corpse,
    /// Player has released spirit and is now a ghost at a graveyard.
    /// For creatures, this means the corpse has despawned.
    Dead,
    /// Transitional: unit is being resurrected this tick. Used to trigger
    /// one-time resurrection side effects. Automatically advances to Alive.
    JustAlived,
}

impl Default for DeathState {
    fn default() -> Self {
        DeathState::Alive
    }
}

/// All mutable state tracked by the death system for a single player.
/// Embedded in the Player struct.
#[derive(Debug, Clone)]
pub struct DeathSystemState {
    /// Current position in the death state machine.
    pub death_state: DeathState,

    /// Countdown timer for corpse expiry. Starts at CORPSE_REPOP_TIME_MS
    /// (360,000 ms = 6 minutes) when the player dies. When it reaches zero
    /// the corpse is automatically converted to bones and the player is
    /// force-released if they have not already released.
    pub death_timer_ms: u32,

    /// World position where the player died and the corpse was placed.
    /// Set on JustDied, cleared on resurrection.
    pub corpse_position: Option<Position>,

    /// Map ID of the corpse. Needed for cross-map validation (e.g. dying
    /// inside an instance but resurrecting outside).
    pub corpse_map_id: Option<u32>,

    /// GUID of the corpse world object, if one has been created.
    pub corpse_guid: Option<ObjectGuid>,

    /// Pending resurrection offer from another player or spirit healer.
    /// Only one offer can be active at a time.
    pub resurrection_data: Option<ResurrectionData>,

    /// Auto-release timer. In battlegrounds this is 30 seconds; in the open
    /// world this is the full CORPSE_REPOP_TIME_MS. When it expires the
    /// player is automatically released as a ghost.
    pub release_timer_ms: u32,

    /// Whether this death was a PvP death (killed by another player).
    /// Affects corpse reclaim delay (30s normal vs 2min PvP).
    pub is_pvp_death: bool,

    /// Unix seconds: the timestamp at which the player's "recent deaths"
    /// streak resets. Populated on each fresh death as `now + 30 minutes`.
    /// Used by `compute_reclaim_delay` to ladder 30s → 60s → 120s as deaths
    /// stack. Zero = no recent deaths. Persisted as `characters.death_expire_time`.
    pub death_expire_time: u64,
}

impl Default for DeathSystemState {
    fn default() -> Self {
        Self {
            death_state: DeathState::Alive,
            death_timer_ms: 0,
            corpse_position: None,
            corpse_map_id: None,
            corpse_guid: None,
            resurrection_data: None,
            release_timer_ms: 0,
            is_pvp_death: false,
            death_expire_time: 0,
        }
    }
}

/// Data attached to a pending resurrection offer.
/// Created when a player casts a resurrection spell or the spirit healer
/// is interacted with.
#[derive(Debug, Clone)]
pub struct ResurrectionData {
    /// GUID of the unit offering the resurrection (player or spirit healer).
    pub resurrector_guid: ObjectGuid,
    /// Where the resurrected player will appear.
    pub location: Position,
    /// Map ID for the resurrection location.
    pub map_id: u32,
    /// Instance ID for validation (prevent cross-instance exploits).
    pub instance_id: u32,
    /// Absolute health to restore on accept.
    pub health: u32,
    /// Absolute mana to restore on accept.
    pub mana: u32,
}

impl ResurrectionData {
    pub fn new(
        resurrector_guid: ObjectGuid,
        location: Position,
        map_id: u32,
        instance_id: u32,
        health: u32,
        mana: u32,
    ) -> Self {
        Self {
            resurrector_guid,
            location,
            map_id,
            instance_id,
            health,
            mana,
        }
    }
}
