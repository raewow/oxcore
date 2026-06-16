mod state;
mod system;

pub use state::DeathState;
pub use system::send_dynamic_flags_update;
pub use system::{process_corpse_decay, process_deaths};
pub use system::{CORPSE_DECAY_BOSS, CORPSE_DECAY_ELITE, CORPSE_DECAY_NORMAL, CORPSE_DECAY_RARE};
pub use system::{
    UNIT_DYNFLAG_DEAD, UNIT_DYNFLAG_LOOTABLE, UNIT_DYNFLAG_TAPPED, UNIT_DYNFLAG_TAPPED_BY_PLAYER,
};
pub use system::{UNIT_FLAG_IMMUNE_TO_PLAYER, UNIT_FLAG_NOT_ATTACKABLE_1};
