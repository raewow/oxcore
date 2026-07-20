//! Battle.net (BGS) login server for modern Classic clients (1.14.x).
//!
//! Runs alongside — not instead of — `oxcore-auth`, which continues to serve the 1.12 realmd
//! protocol. The two share only the account database.

pub mod certs;
pub mod config;
pub mod gen_certs;
pub mod rest;

/// SRP6v2 lives in `oxcore-shared` (the account repository needs it too); re-exported here so
/// `crate::srp6v2` continues to resolve within this crate.
pub use oxcore_shared::crypto::srp6v2;
pub mod run;
pub mod server;
pub mod tls;

pub use run::{serve, BnetServer};

pub mod shared {
    pub use oxcore_shared::*;
}
