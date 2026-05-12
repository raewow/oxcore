mod state;
mod system;

pub use state::DeathState;
pub use system::{process_deaths, process_corpse_decay};
pub use system::{CORPSE_DECAY_NORMAL, CORPSE_DECAY_RARE, CORPSE_DECAY_ELITE, CORPSE_DECAY_BOSS};
pub use system::{UNIT_DYNFLAG_DEAD, UNIT_DYNFLAG_LOOTABLE, UNIT_DYNFLAG_TAPPED_BY_PLAYER, UNIT_DYNFLAG_TAPPED};
pub use system::{UNIT_FLAG_NOT_ATTACKABLE_1, UNIT_FLAG_IMMUNE_TO_PLAYER};
pub use system::send_dynamic_flags_update;
