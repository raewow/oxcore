//! Movement subsystem - player movement state and processing

pub mod state;
pub mod system;
pub mod validator;

#[cfg(test)]
mod tests;

pub use state::MovementState;
pub use system::MovementSystem;
