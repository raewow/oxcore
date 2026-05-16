//! Pathfinding types - path results, flags, and height queries

use crate::shared::protocol::Position;

/// Path calculation result
#[derive(Debug, Clone)]
pub enum PathResult {
    /// Full path found
    Complete(Vec<Position>),
    /// Partial path (couldn't reach destination)
    Partial(Vec<Position>),
    /// No path possible
    NoPath,
    /// Path is a straight line (no obstacles)
    StraightLine(Position, Position),
}

impl PathResult {
    pub fn is_complete(&self) -> bool {
        matches!(
            self,
            PathResult::Complete(_) | PathResult::StraightLine(_, _)
        )
    }

    pub fn waypoints(&self) -> Vec<Position> {
        match self {
            PathResult::Complete(wp) => wp.clone(),
            PathResult::Partial(wp) => wp.clone(),
            PathResult::StraightLine(start, end) => vec![*start, *end],
            PathResult::NoPath => vec![],
        }
    }
}

/// Path type flags (matches MaNGOS PathType)
pub mod path_flags {
    pub const PATH_NORMAL: u32 = 0x0001;
    pub const PATH_NOT_USING_PATH: u32 = 0x0002;
    pub const PATH_INCOMPLETE: u32 = 0x0004;
    pub const PATH_FARFROM_START: u32 = 0x0008;
    pub const PATH_FARFROM_END: u32 = 0x0010;
}

/// Height query result
#[derive(Debug, Clone, Copy)]
pub struct HeightResult {
    pub height: f32,
    pub found: bool,
}
