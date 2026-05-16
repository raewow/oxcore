//! Game-level PathFinder - high-level wrapper for movement generators
//!
//! Wraps the low-level MMap PathFinder with a simpler API for use by
//! movement generators (chase, random, waypoint, etc.)

use super::pathfinder::PathFinder as MMapPathFinder;
use super::types::PathResult;
use crate::shared::protocol::Position;
use std::sync::Arc;

/// Game-level PathFinder (wraps low-level MMap PathFinder)
pub struct GamePathFinder {
    mmap_pathfinder: Option<Arc<MMapPathFinder>>,
}

impl GamePathFinder {
    /// Create PathFinder with MMap integration
    pub fn with_mmap(mmap_pathfinder: Arc<MMapPathFinder>) -> Self {
        Self {
            mmap_pathfinder: Some(mmap_pathfinder),
        }
    }

    /// Create PathFinder without MMap (straight-line only)
    pub fn without_mmap() -> Self {
        Self {
            mmap_pathfinder: None,
        }
    }

    /// Calculate path (delegates to MMap PathFinder)
    pub fn calculate_path(&self, map_id: u32, start: Position, end: Position) -> PathResult {
        if let Some(ref pathfinder) = self.mmap_pathfinder {
            pathfinder.calculate_path(map_id, start, end)
        } else {
            // No MMap - only straight line
            PathResult::StraightLine(start, end)
        }
    }

    /// Check if MMap pathfinding is available
    pub fn has_mmap(&self) -> bool {
        self.mmap_pathfinder.is_some()
    }
}
