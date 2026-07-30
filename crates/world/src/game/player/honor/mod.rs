//! PvP honor / kill-credit system.
//!
//! Tracks damage contributors on each player so that when they die to enemy
//! players, all attackers within a 60-second window receive honor credit.

pub mod contributor;
pub mod system;

pub use contributor::{Contributor, ContributorTracker, CONTRIBUTOR_WINDOW_SECS};
pub use system::HonorSystem;
