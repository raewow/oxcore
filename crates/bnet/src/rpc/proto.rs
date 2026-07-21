//! Generated BGS protobuf types (from `proto/bgs.proto`, compiled by `build.rs`).

// All messages share the `bgs.protocol` package, so prost emits a single flat module.
include!(concat!(env!("OUT_DIR"), "/bgs.protocol.rs"));
