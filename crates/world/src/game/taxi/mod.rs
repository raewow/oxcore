//! Flight paths: the taxi network (nodes, direct paths and their points) and
//! each player's taxi state (known nodes, the journey in progress).
//!
//! Nodes come from `world.taxi_nodes` (newest row at or below the vanilla
//! build), paths and their points from `TaxiPath.dbc` / `TaxiPathNode.dbc`,
//! and the hand-over points between consecutive paths of a multi-hop journey
//! from `world.taxi_path_transitions`.

pub mod flight;

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use anyhow::Result;
use oxcore_shared::game::chat::Team;
use oxcore_shared::game::taxi::TaxiMask;
use oxcore_shared::protocol::Position;
use sqlx::PgPool;

use crate::dbc::DbcManager;

/// Client build whose taxi nodes are used.
const VANILLA_CLIENT_BUILD: i32 = 5875;
/// Spell effect that starts a scripted flight (`SPELL_EFFECT_SEND_TAXI`).
pub const SPELL_EFFECT_SEND_TAXI: u32 = 123;

/// A flight master's landing point.
#[derive(Debug, Clone)]
pub struct TaxiNode {
    pub id: u32,
    pub map_id: u32,
    pub position: Position,
    pub name: String,
    /// Mount creature for Horde (0) and Alliance (1) riders.
    pub mount_creature: [u32; 2],
}

/// One point of a taxi path.
#[derive(Debug, Clone, Copy)]
pub struct TaxiPathPoint {
    pub path: u32,
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl TaxiPathPoint {
    pub fn position(&self) -> Position {
        Position::new(self.x, self.y, self.z, 0.0)
    }
}

/// Where a multi-hop journey leaves one path and joins the next.
#[derive(Debug, Clone, Copy)]
struct Transition {
    out_path: u32,
    in_node: usize,
    out_node: usize,
}

/// Holds the taxi network, swapped in whole once loaded so readers can keep
/// a snapshot across awaits.
#[derive(Default)]
pub struct TaxiManager {
    network: parking_lot::RwLock<std::sync::Arc<TaxiNetwork>>,
}

impl TaxiManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// The current network.
    pub fn network(&self) -> std::sync::Arc<TaxiNetwork> {
        self.network.read().clone()
    }

    /// Load the network; `spell_flight_paths` are the paths started by
    /// scripted taxi spells.
    pub async fn load(
        &self,
        world_db: &PgPool,
        dbc: &parking_lot::RwLock<DbcManager>,
        spell_flight_paths: &HashSet<u32>,
    ) -> Result<()> {
        let network = TaxiNetwork::load(world_db, dbc, spell_flight_paths).await?;
        *self.network.write() = std::sync::Arc::new(network);
        Ok(())
    }
}

/// The taxi network.
#[derive(Default)]
pub struct TaxiNetwork {
    nodes: HashMap<u32, TaxiNode>,
    /// source node -> destination node -> (path id, price)
    paths: HashMap<u32, BTreeMap<u32, (u32, u32)>>,
    /// path id -> its points in order
    path_points: HashMap<u32, Vec<TaxiPathPoint>>,
    /// path id -> transitions out of it
    transitions: HashMap<u32, Vec<Transition>>,
    /// Nodes that belong to the flight-master network (not only reached by
    /// scripted spell flights).
    network: TaxiMask,
}

impl TaxiNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load the network from the DBC paths and the world database nodes.
    /// `spell_flight_paths` are the paths started by scripted taxi spells.
    pub async fn load(
        world_db: &PgPool,
        dbc: &parking_lot::RwLock<DbcManager>,
        spell_flight_paths: &HashSet<u32>,
    ) -> Result<Self> {
        let nodes = sqlx::query_as::<_, (i64, i64, f32, f32, f32, String, i64, i64)>(
            "SELECT DISTINCT ON (id) id::bigint, map_id::bigint, x, y, z, name,
                    mount_creature_id1::bigint, mount_creature_id2::bigint
             FROM world.taxi_nodes
             WHERE build <= $1
             ORDER BY id, build DESC",
        )
        .bind(VANILLA_CLIENT_BUILD)
        .fetch_all(world_db)
        .await?;
        let transitions = sqlx::query_as::<_, (i64, i64, i64, i64)>(
            "SELECT in_path::bigint, out_path::bigint, in_node::bigint, out_node::bigint
             FROM world.taxi_path_transitions WHERE build_min <= $1",
        )
        .bind(VANILLA_CLIENT_BUILD)
        .fetch_all(world_db)
        .await?;

        let mut mgr = Self::new();
        let dbc = dbc.read();
        for (_, path) in dbc.taxi_path.entries() {
            mgr.paths
                .entry(path.from)
                .or_default()
                .insert(path.to, (path.id, path.price));
        }

        let mut points: HashMap<u32, Vec<(u32, TaxiPathPoint)>> = HashMap::new();
        for (_, point) in dbc.taxi_path_node.entries() {
            points.entry(point.path).or_default().push((
                point.index,
                TaxiPathPoint {
                    path: point.path,
                    map_id: point.map_id,
                    x: point.x,
                    y: point.y,
                    z: point.z,
                },
            ));
        }
        for (path, mut list) in points {
            list.sort_by_key(|(index, _)| *index);
            mgr.path_points
                .insert(path, list.into_iter().map(|(_, point)| point).collect());
        }

        drop(dbc);
        for (id, map_id, x, y, z, name, horde, alliance) in nodes {
            let id = u32::try_from(id)?;
            mgr.nodes.insert(
                id,
                TaxiNode {
                    id,
                    map_id: u32::try_from(map_id)?,
                    position: Position::new(x, y, z, 0.0),
                    name,
                    mount_creature: [u32::try_from(horde)?, u32::try_from(alliance)?],
                },
            );
        }

        for (in_path, out_path, in_node, out_node) in transitions {
            mgr.transitions
                .entry(u32::try_from(in_path)?)
                .or_default()
                .push(Transition {
                    out_path: u32::try_from(out_path)?,
                    in_node: usize::try_from(in_node)?,
                    out_node: usize::try_from(out_node)?,
                });
        }

        mgr.build_network(spell_flight_paths);
        tracing::info!(
            "Loaded {} taxi nodes, {} paths",
            mgr.nodes.len(),
            mgr.path_points.len()
        );
        Ok(mgr)
    }

    /// A node is on the network unless every path out of it is a scripted
    /// spell flight.
    fn build_network(&mut self, spell_flight_paths: &HashSet<u32>) {
        self.network.clear();
        for &id in self.nodes.keys() {
            let scripted_only = self.paths.get(&id).is_some_and(|out| {
                !out.is_empty()
                    && out
                        .values()
                        .all(|(path, _)| spell_flight_paths.contains(path))
            });
            if !scripted_only && id > 0 {
                self.network.set((id - 1) as usize);
            }
        }
    }

    pub fn node(&self, id: u32) -> Option<&TaxiNode> {
        self.nodes.get(&id)
    }

    /// The flight-master network as a mask (bit `n - 1` for node `n`).
    pub fn network_mask(&self) -> &TaxiMask {
        &self.network
    }

    /// Direct path between two nodes: `(path id, price)`.
    pub fn path(&self, source: u32, destination: u32) -> Option<(u32, u32)> {
        self.paths.get(&source)?.get(&destination).copied()
    }

    /// The source and destination of a path.
    pub fn path_ends(&self, path: u32) -> Option<(u32, u32)> {
        self.paths.iter().find_map(|(&from, out)| {
            out.iter()
                .find(|(_, &(id, _))| id == path)
                .map(|(&to, _)| (from, to))
        })
    }

    pub fn path_points(&self, path: u32) -> Option<&[TaxiPathPoint]> {
        self.path_points.get(&path).map(Vec::as_slice)
    }

    /// The network node nearest a spot on a map that serves `team`.
    pub fn nearest_node(&self, map_id: u32, position: Position, team: Team) -> Option<u32> {
        let side = team_index(team)?;
        self.nodes
            .values()
            .filter(|node| {
                node.map_id == map_id
                    && node.mount_creature[side] != 0
                    && node.id > 0
                    && self.network.is_set((node.id - 1) as usize)
            })
            .min_by(|a, b| {
                distance_sq(a.position, position).total_cmp(&distance_sq(b.position, position))
            })
            .map(|node| node.id)
    }

    /// The mount creature for flying from a node. Off the flight-master path
    /// (spells, scripts) the other side's mount stands in.
    pub fn mount_creature(&self, node: u32, team: Team, allow_other_side: bool) -> Option<u32> {
        let node = self.nodes.get(&node)?;
        let side = team_index(team)?;
        let own = node.mount_creature[side];
        if own != 0 {
            return Some(own);
        }
        let other = node.mount_creature[1 - side];
        (allow_other_side && other != 0).then_some(other)
    }

    /// Where a journey moves from `in_path` to `out_path`: the last point used
    /// on the first and the first point used on the second.
    fn transition(&self, in_path: u32, out_path: u32) -> (usize, usize) {
        let found = self
            .transitions
            .get(&in_path)
            .and_then(|list| list.iter().find(|t| t.out_path == out_path));
        let in_len = self.path_points.get(&in_path).map_or(0, Vec::len);
        match found {
            Some(t) if t.in_node != 0 && t.out_node != 0 => (t.in_node, t.out_node),
            Some(t) if t.in_node != 0 => (t.in_node, 1),
            Some(t) if t.out_node != 0 => (in_len.saturating_sub(2), t.out_node),
            _ => {
                if found.is_none() {
                    tracing::debug!(
                        "No taxi path transition between paths {} and {}",
                        in_path,
                        out_path
                    );
                }
                (in_len.saturating_sub(2), 1)
            }
        }
    }

    /// The points flown over a journey through `nodes`: the whole of the
    /// first path for a single hop, joined at each transition otherwise.
    /// None when any hop has no path.
    pub fn journey_points(&self, nodes: &[u32]) -> Option<Vec<TaxiPathPoint>> {
        let mut paths = Vec::with_capacity(nodes.len().saturating_sub(1));
        for hop in nodes.windows(2) {
            paths.push(self.path(hop[0], hop[1])?.0);
        }
        let mut points = Vec::new();
        let mut from = 0usize;
        for (i, &path) in paths.iter().enumerate() {
            let list = self.path_points.get(&path)?;
            match paths.get(i + 1) {
                Some(&next) => {
                    let (in_node, out_node) = self.transition(path, next);
                    let until = in_node.min(list.len().saturating_sub(1));
                    if from <= until {
                        points.extend_from_slice(&list[from..=until]);
                    }
                    from = out_node;
                }
                None => {
                    if from < list.len() {
                        points.extend_from_slice(&list[from..]);
                    }
                }
            }
        }
        (!points.is_empty()).then_some(points)
    }

    /// The cheapest-by-distance route between two nodes over the nodes in
    /// `known` (1.14 clients name only the destination).
    pub fn route(&self, from: u32, to: u32, known: &TaxiMask) -> Option<Vec<u32>> {
        if self.path(from, to).is_some() {
            return Some(vec![from, to]);
        }
        let usable = |node: u32| node > 0 && known.is_set((node - 1) as usize);
        let mut dist: HashMap<u32, f32> = HashMap::from([(from, 0.0)]);
        let mut previous: HashMap<u32, u32> = HashMap::new();
        let mut open: Vec<u32> = vec![from];
        let mut done: HashSet<u32> = HashSet::new();
        while let Some(index) = open
            .iter()
            .enumerate()
            .min_by(|a, b| dist[a.1].total_cmp(&dist[b.1]))
            .map(|(i, _)| i)
        {
            let node = open.swap_remove(index);
            if node == to {
                break;
            }
            if !done.insert(node) {
                continue;
            }
            for (&next, &(path, _)) in self.paths.get(&node).into_iter().flatten() {
                if !usable(next) || done.contains(&next) {
                    continue;
                }
                let step = self.path_length(path);
                let candidate = dist[&node] + step;
                if dist.get(&next).map_or(true, |&d| candidate < d) {
                    dist.insert(next, candidate);
                    previous.insert(next, node);
                    open.push(next);
                }
            }
        }
        if !previous.contains_key(&to) {
            return None;
        }
        let mut route = vec![to];
        let mut node = to;
        while let Some(&prev) = previous.get(&node) {
            route.push(prev);
            node = prev;
        }
        route.reverse();
        Some(route)
    }

    /// 2D length of a path.
    fn path_length(&self, path: u32) -> f32 {
        self.path_points.get(&path).map_or(f32::MAX, |points| {
            points
                .windows(2)
                .map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y))
                .sum::<f32>()
                .max(1.0)
        })
    }
}

fn team_index(team: Team) -> Option<usize> {
    match team {
        Team::Horde => Some(0),
        Team::Alliance => Some(1),
        _ => None,
    }
}

fn distance_sq(a: Position, b: Position) -> f32 {
    (a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)
}

/// A player's taxi state.
#[derive(Debug, Clone, Default)]
pub struct PlayerTaxi {
    /// Flight points the player has discovered.
    pub known: TaxiMask,
    /// Nodes of the journey still ahead; the first is where the current hop
    /// starts. Empty when not flying.
    pub destinations: VecDeque<u32>,
    /// Reputation discount on each hop's price.
    pub discount: f32,
    /// The flight in progress.
    pub flight: Option<flight::Flight>,
}

impl PlayerTaxi {
    pub fn knows(&self, node: u32) -> bool {
        node > 0 && self.known.is_set((node - 1) as usize)
    }

    /// Learn a node; false when it was already known.
    pub fn learn(&mut self, node: u32) -> bool {
        if node == 0 || self.knows(node) {
            return false;
        }
        self.known.set((node - 1) as usize);
        true
    }

    pub fn is_flying(&self) -> bool {
        self.flight.is_some()
    }

    /// The known-node mask as stored: eight words separated by spaces.
    pub fn mask_to_string(&self) -> String {
        self.known
            .as_array()
            .iter()
            .map(|word| word.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Load the stored mask, keeping only nodes still on the network.
    pub fn load_mask(&mut self, stored: &str, network: &TaxiMask) {
        let network = network.as_array();
        for (i, word) in stored.split_whitespace().take(8).enumerate() {
            self.known.0[i] = word.parse::<u32>().unwrap_or(0) & network[i];
        }
    }

    /// The hop in progress as stored ("source destination "), or empty.
    pub fn journey_to_string(&self) -> String {
        if self.destinations.len() < 2 {
            return String::new();
        }
        format!("{} {} ", self.destinations[0], self.destinations[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> TaxiNetwork {
        let mut mgr = TaxiNetwork::new();
        for (id, x) in [(1u32, 0.0f32), (2, 100.0), (3, 200.0), (4, 300.0)] {
            mgr.nodes.insert(
                id,
                TaxiNode {
                    id,
                    map_id: 0,
                    position: Position::new(x, 0.0, 0.0, 0.0),
                    name: String::new(),
                    mount_creature: [10, 20],
                },
            );
        }
        let line = |path: u32, from: f32, to: f32| {
            (0..=4)
                .map(|i| TaxiPathPoint {
                    path,
                    map_id: 0,
                    x: from + (to - from) * i as f32 / 4.0,
                    y: 0.0,
                    z: 0.0,
                })
                .collect::<Vec<_>>()
        };
        mgr.paths.entry(1).or_default().insert(2, (11, 100));
        mgr.paths.entry(2).or_default().insert(3, (12, 50));
        mgr.paths.entry(3).or_default().insert(4, (13, 70));
        mgr.path_points.insert(11, line(11, 0.0, 100.0));
        mgr.path_points.insert(12, line(12, 100.0, 200.0));
        mgr.path_points.insert(13, line(13, 200.0, 300.0));
        mgr.build_network(&HashSet::new());
        mgr
    }

    #[test]
    fn route_crosses_known_nodes_only() {
        let mgr = manager();
        let mut known = TaxiMask::new();
        for node in [1, 2, 3] {
            known.set(node - 1);
        }
        assert_eq!(mgr.route(1, 3, &known), Some(vec![1, 2, 3]));
        // Node 3 unknown: no way to 4.
        let mut partial = TaxiMask::new();
        partial.set(0);
        partial.set(1);
        partial.set(3);
        assert_eq!(mgr.route(1, 4, &partial), None);
    }

    #[test]
    fn multi_hop_journey_joins_paths_at_the_default_transition() {
        let mgr = manager();
        let single = mgr.journey_points(&[1, 2]).unwrap();
        assert_eq!(single.len(), 5);
        let points = mgr.journey_points(&[1, 2, 3]).unwrap();
        // Path 11 points 0..=3, then path 12 from point 1.
        assert_eq!(points.len(), 4 + 4);
        assert_eq!(points[3].path, 11);
        assert_eq!(points[4].path, 12);
        assert!(mgr.journey_points(&[1, 3]).is_none());
    }

    #[test]
    fn nearest_node_serves_only_the_team_with_a_mount() {
        let mut mgr = manager();
        mgr.nodes.get_mut(&2).unwrap().mount_creature = [0, 20];
        let at = Position::new(95.0, 0.0, 0.0, 0.0);
        assert_eq!(mgr.nearest_node(0, at, Team::Alliance), Some(2));
        assert_eq!(mgr.nearest_node(0, at, Team::Horde), Some(1));
    }

    #[test]
    fn stored_mask_round_trips_and_drops_unknown_network_nodes() {
        let mgr = manager();
        let mut taxi = PlayerTaxi::default();
        taxi.load_mask("15 0 0 0 0 0 0 0", mgr.network_mask());
        assert!(taxi.knows(1) && taxi.knows(4));
        taxi.load_mask("4294967295 1 0 0 0 0 0 0", mgr.network_mask());
        assert!(!taxi.knows(5));
        assert_eq!(taxi.mask_to_string(), "15 0 0 0 0 0 0 0");
    }
}
