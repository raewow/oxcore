pub mod common;
pub mod config;
pub mod console;
pub mod database;
pub mod game;
pub mod messages;
pub mod protocol;

pub mod shared {
    pub use crate::{common, config, console, database, game, messages, protocol};
}
