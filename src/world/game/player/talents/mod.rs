//! Talent System
//!
//! Manages player talent trees including:
//! - Talent point allocation and validation
//! - Prerequisite checking
//! - Passive aura application from talents
//! - Spell modifier application from talents
//! - Talent reset with escalating cost

pub mod dbc;
pub mod effects;
pub mod points;
pub mod reset;
pub mod state;
pub mod system;
pub mod validation;

pub use dbc::{TalentInfo, TalentStore, TalentTabInfo};
pub use state::{
    TalentState, MAX_TALENT_COLUMNS, MAX_TALENT_RANK, MAX_TALENT_ROWS, MAX_TALENT_TABS,
    POINTS_PER_ROW,
};
pub use system::TalentSystem;
pub use validation::{validate_learn_talent, TalentLearnResult};
