//! PvP honour: contribution points, honourable and dishonourable kills,
//! ranks and the weekly ranking.
//!
//! Damage between enemy players is tracked on the victim so that when they
//! die, the attackers (and their groups) of the last minute share the honour.

pub mod contributor;
pub mod rank;
pub mod system;

pub use contributor::{Contributor, ContributorTracker, CONTRIBUTOR_WINDOW_SECS};
pub use rank::HonorRankInfo;
pub use system::{
    add_honor, load, on_creature_killed, save, send_fields, weekly_maintenance, HonorState,
    HonorSystem, HonorType,
};
