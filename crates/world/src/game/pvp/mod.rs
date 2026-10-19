//! Player-versus-player state shared by battlegrounds and the open world:
//! the PvP flag, objective scripts, and the world states and spawn events
//! those scripts drive.

pub mod events;
pub mod flag;
pub mod objective;
pub mod scope;

pub use flag::PvpState;
pub use scope::{Scope, ScopeStates};
