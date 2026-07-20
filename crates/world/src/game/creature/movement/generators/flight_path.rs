//! Flight-path (taxi) navigation.
//!
//! A taxi route can span several maps. The client is flown one map-leg at a time, so the
//! server needs to know where the current leg ends and, after the loading screen between
//! maps, which node to resume from. That path-index logic is pure and lives here.
//!
//! The player-facing lifecycle of `FlightPathMovementGenerator` - taxi state, money
//! charged per leg, mount and PvP handling, launching the spline - is not ported: none of
//! that infra exists yet. This is the navigation core it will build on.

use oxcore_shared::game::TaxiNode;
use oxcore_shared::protocol::Position;

/// Tracks progress along a multi-map taxi route.
#[derive(Debug, Clone, Default)]
pub struct FlightPathNavigator {
    path: Vec<TaxiNode>,
    current_node: usize,
}

impl FlightPathNavigator {
    pub fn new(path: Vec<TaxiNode>) -> Self {
        Self {
            path,
            current_node: 0,
        }
    }

    pub fn current_node(&self) -> usize {
        self.current_node
    }

    pub fn set_current_node(&mut self, node: usize) {
        self.current_node = node;
    }

    pub fn path(&self) -> &[TaxiNode] {
        &self.path
    }

    /// Index one past the last node on the current node's map (`GetPathAtMapEnd`).
    ///
    /// This is where the current flight leg stops; the flight continues from there on the
    /// next map after the loading screen. Returns the path length when the current node is
    /// out of range or the whole remainder is on one map.
    pub fn path_at_map_end(&self) -> usize {
        if self.current_node >= self.path.len() {
            return self.path.len();
        }

        let current_map = self.path[self.current_node].map_id;

        self.path[self.current_node..]
            .iter()
            .position(|node| node.map_id != current_map)
            .map(|offset| self.current_node + offset)
            .unwrap_or(self.path.len())
    }

    /// Resume point after crossing a map boundary (`SetCurrentNodeAfterTeleport`).
    ///
    /// Advances the current node to the first node whose map differs from the route's
    /// starting map, and reports whether such a boundary was found. An empty path or a
    /// single-map route leaves the current node untouched.
    pub fn set_current_node_after_teleport(&mut self) -> bool {
        let Some(first) = self.path.first() else {
            return false;
        };
        let start_map = first.map_id;

        if let Some(index) = self.path[1..]
            .iter()
            .position(|node| node.map_id != start_map)
        {
            self.current_node = index + 1;
            return true;
        }

        false
    }

    /// Position of the current node, used to place the unit when its movement is rebuilt
    /// (`GetResetPosition`). `None` when the current node is out of range.
    pub fn reset_position(&self) -> Option<Position> {
        self.path.get(self.current_node).map(|node| Position {
            x: node.x,
            y: node.y,
            z: node.z,
            o: 0.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(map_id: u32, x: f32) -> TaxiNode {
        TaxiNode {
            id: 0,
            name: String::new(),
            map_id,
            x,
            y: 0.0,
            z: 0.0,
            next_nodes: Vec::new(),
            mount_creature_id: 0,
            cost: 0,
        }
    }

    /// A route that starts on map 0 and crosses onto map 1 at index 2.
    fn cross_map_route() -> Vec<TaxiNode> {
        vec![
            node(0, 0.0),
            node(0, 10.0),
            node(1, 20.0),
            node(1, 30.0),
        ]
    }

    #[test]
    fn path_at_map_end_stops_at_the_first_map_change() {
        let nav = FlightPathNavigator::new(cross_map_route());
        // From the start, the leg on map 0 ends where map 1 begins.
        assert_eq!(nav.path_at_map_end(), 2);
    }

    #[test]
    fn path_at_map_end_returns_length_for_a_single_map_remainder() {
        let mut nav = FlightPathNavigator::new(cross_map_route());
        // Already on the map-1 leg: nothing after it changes map.
        nav.set_current_node(2);
        assert_eq!(nav.path_at_map_end(), 4);
    }

    #[test]
    fn path_at_map_end_handles_an_out_of_range_node() {
        let mut nav = FlightPathNavigator::new(cross_map_route());
        nav.set_current_node(99);
        assert_eq!(nav.path_at_map_end(), 4);

        let empty = FlightPathNavigator::default();
        assert_eq!(empty.path_at_map_end(), 0);
    }

    #[test]
    fn teleport_advances_to_the_first_node_on_the_next_map() {
        let mut nav = FlightPathNavigator::new(cross_map_route());

        assert!(nav.set_current_node_after_teleport());
        assert_eq!(nav.current_node(), 2);
    }

    #[test]
    fn teleport_leaves_a_single_map_route_untouched() {
        let mut nav = FlightPathNavigator::new(vec![node(0, 0.0), node(0, 10.0)]);

        assert!(!nav.set_current_node_after_teleport());
        assert_eq!(nav.current_node(), 0);

        let mut empty = FlightPathNavigator::default();
        assert!(!empty.set_current_node_after_teleport());
    }

    #[test]
    fn reset_position_reports_the_current_node_coordinates() {
        let mut nav = FlightPathNavigator::new(cross_map_route());
        nav.set_current_node(3);

        let pos = nav.reset_position().unwrap();
        assert_eq!(pos.x, 30.0);

        nav.set_current_node(99);
        assert!(nav.reset_position().is_none());
    }
}
