//! Spatial Tree Construction for VMap Assembly
//!
//! This module handles the construction of spatial acceleration structures
//! for efficient collision detection and line-of-sight queries.
//!
//! - BIH (Bounding Interval Hierarchy) - Used for server-compatible output
//! - BVH (Bounding Volume Hierarchy) - Legacy implementation

pub mod bih;
pub mod builder;
pub mod output;
pub mod structures;

pub use bih::{BIH, BuildStats as BIHBuildStats};
pub use output::VMTREE_MAGIC;
pub use structures::{BVHNode, BVHTree, TriangleData};
