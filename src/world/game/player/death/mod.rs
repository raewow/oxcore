//! Death and Resurrection System
//!
//! This module implements the complete death and resurrection pipeline:
//! 1. Death state machine with transitional states (JustDied, Corpse, Dead, JustAlived)
//! 2. Ghost form with invisibility to living players and water walking
//! 3. Corpse creation, tracking, persistence, and bone conversion
//! 4. Graveyard selection by zone, faction, and distance
//! 5. Multiple resurrection paths (corpse run, spirit healer, player spell, self-res)
//! 6. Resurrection sickness with level-scaled duration
//! 7. Durability loss on death and spirit healer resurrection

pub mod corpse;
pub mod durability;
pub mod flow;
pub mod ghost;
pub mod graveyard;
pub mod resurrect;
pub mod sickness;
pub mod state;
pub mod system;

// Re-export main types
pub use corpse::{Corpse, CorpseType, create_corpse_from_player, CORPSE_RECLAIM_RADIUS};
pub use durability::{
    apply_death_durability_loss,
    apply_spirit_healer_durability_loss,
    should_apply_durability_loss,
    DEATH_DURABILITY_LOSS,
    SPIRIT_HEALER_DURABILITY_LOSS,
};
pub use flow::{
    can_reclaim_corpse,
    can_release_spirit,
    get_corpse_reclaim_delay,
    get_release_timer_ms,
    is_within_corpse_reclaim_range,
    tick_death_timer,
    CORPSE_RECLAIM_DELAY_NORMAL,
    CORPSE_RECLAIM_DELAY_PVP,
    CORPSE_REPOP_TIME_MS,
    GHOST_SPEED_MULTIPLIER,
    GHOST_SPEED_MULTIPLIER_BG,
    PLAYER_FLAGS_GHOST,
    SPELL_AURA_GHOST,
    SPELL_WISP_FORM,
    UNIT_FLAG_DISABLE_MOVE,
};
pub use ghost::{
    build_player_repop,
    get_ghost_speed_multiplier,
    remove_ghost_form,
};
pub use graveyard::{
    find_closest_graveyard,
    team_from_race,
    GraveyardData,
    GraveyardManager,
    FACTION_ALLIANCE,
    FACTION_HORDE,
    FACTION_NONE,
    TEAM_ALLIANCE,
    TEAM_HORDE,
    TEAM_BOTH,
};
pub use resurrect::{
    decline_resurrection,
    is_resurrection_requested_by,
    offer_resurrection,
    resurrect_at_corpse,
    resurrect_at_spirit_healer,
    resurrect_from_spell,
    ResurrectionMethod,
};
pub use sickness::{
    compute_resurrection_sickness,
    get_resurrection_sickness_duration,
    get_resurrection_sickness_spell_id,
    DEATH_SICKNESS_LEVEL,
    SPELL_RESURRECTION_SICKNESS,
};
pub use state::{DeathState, DeathSystemState, ResurrectionData};
pub use system::DeathSystem;
