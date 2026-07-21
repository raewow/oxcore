//! The BGS protobuf-RPC channel served on :1119.
//!
//! After the client authenticates over the REST login (see [`crate::rest`]) it opens a TLS
//! connection here and speaks length-prefixed protobuf frames. This module owns the frame
//! codec ([`framing`]), the service handlers ([`services`]), and the per-connection loop
//! ([`session`]).

pub mod framing;
pub mod proto;
pub mod services;
pub mod session;
