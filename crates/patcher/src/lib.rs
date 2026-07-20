//! Patches a World of Warcraft 1.14.x client so it connects to an oxcore server.
//!
//! Two modifications are needed:
//!
//! 1. **Portal redirect** — the client hardcodes `.actual.battle.net` as the suffix of the
//!    `portal` value in `WTF/Config.wtf`. Replacing it with `.localhost` (or your own domain)
//!    sends the login request to a server you control.
//! 2. **Signature modulus** — the client verifies its embedded certificate bundle against a
//!    256-byte RSA modulus baked into the executable. Replacing it with ours lets us sign a
//!    bundle naming our own host, so the client trusts our TLS certificate without any public
//!    certificate authority being involved.
//!
//! Everything is written to a copy; the original executable is never modified in place.

pub mod patch;
pub mod patterns;
pub mod scan;

pub use patch::{apply, Patch};
