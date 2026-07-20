//! Movement subsystem - player movement state and processing

pub mod packet_sender;
pub mod state;
pub mod system;
pub mod validator;

#[cfg(test)]
mod tests;

pub use packet_sender::MovementControllerSender;
pub use state::MovementState;
pub use system::MovementSystem;
