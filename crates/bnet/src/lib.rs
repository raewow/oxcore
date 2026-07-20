//! Battle.net (BGS) login server for modern Classic clients (1.14.x).
//!
//! Runs alongside — not instead of — `oxcore-auth`, which continues to serve the 1.12 realmd
//! protocol. The two share only the account database.

pub mod config;
pub mod rest;
pub mod run;
pub mod server;
pub mod tls;

pub use run::{serve, BnetServer};

pub mod shared {
    pub use oxcore_shared::*;
}
