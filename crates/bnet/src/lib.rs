//! Battle.net (BGS) login server for modern Classic clients (1.14.x).
//!
//! Runs alongside — not instead of — `oxcore-auth`, which continues to serve the 1.12 realmd
//! protocol. The two share only the account database.

pub mod certs;
pub mod config;
pub mod gen_certs;
pub mod rest;
pub mod srp6v2;
pub mod run;
pub mod server;
pub mod tls;

pub use run::{serve, BnetServer};

pub mod shared {
    pub use oxcore_shared::*;
}
