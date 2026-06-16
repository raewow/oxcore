pub mod common;
pub mod config;
pub mod console;
pub mod context;
pub mod database;
pub mod init;
pub mod logging;
pub mod metrics;
pub mod patch;
pub mod protocol;
pub mod realm;
pub mod run;
pub mod server;

pub use run::{serve, AuthMetrics};

pub mod shared {
    pub use oxcore_shared::*;
}
