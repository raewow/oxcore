//! M2 (Model) File Parsing and Conversion
//!
//! M2 files contain 3D model geometry for creatures, objects, and doodads.
//! This module extracts collision geometry for VMap generation.

pub mod converter;
pub mod parser;
pub mod structures;

pub use structures::{M2File, M2Header, M2SkinProfile, M2Vertex};
