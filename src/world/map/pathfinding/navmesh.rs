//! Navigation mesh with Rust-native A* pathfinding
//!
//! Provides polygon-based pathfinding using the `pathfinding` crate.
//! Parses Detour .mmtile format and uses A* through polygon neighbors.

use super::types::PathResult;
use crate::shared::protocol::Position;
use std::collections::HashMap;

const INVALID_POLY: u16 = 0xFFFF;

/// Wrapper type for f32 cost that implements Ord (required by pathfinding crate)
#[derive(Debug, Clone, Copy, PartialEq)]
struct Cost(f32);

impl Eq for Cost {}

impl PartialOrd for Cost {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for Cost {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl std::ops::Add for Cost {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Cost(self.0 + other.0)
    }
}

impl num_traits::Zero for Cost {
    fn zero() -> Self {
        Cost(0.0)
    }
    fn is_zero(&self) -> bool {
        self.0 == 0.0
    }
}

/// Navigation mesh vertex
#[derive(Debug, Clone, Copy)]
pub struct NavMeshVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Navigation mesh polygon
#[derive(Debug, Clone)]
pub struct NavMeshPoly {
    /// Indices into vertices array
    pub vertex_indices: Vec<u16>,
    /// Neighbor polygon indices (INVALID_POLY = no neighbor)
    pub neighbors: Vec<u16>,
    /// Polygon flags (water, door, etc.)
    pub flags: u16,
    /// Area type
    pub area: u8,
    /// Polygon reference ID
    pub poly_ref: u32,
}

/// Navigation mesh header data
#[derive(Debug, Clone)]
struct NavMeshHeader {
    pub vert_count: u32,
    pub poly_count: u32,
}

/// Navigation mesh (parses Detour .mmtile format)
pub struct NavMesh {
    /// Polygon vertices
    vertices: Vec<NavMeshVertex>,
    /// Polygons with neighbor connections
    polygons: Vec<NavMeshPoly>,
    /// Polygon reference map (poly_ref -> polygon index)
    poly_ref_map: HashMap<u32, usize>,
    /// Tile header data
    header: NavMeshHeader,
}

impl NavMesh {
    /// Create a new empty navmesh
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            polygons: Vec::new(),
            poly_ref_map: HashMap::new(),
            header: NavMeshHeader {
                vert_count: 0,
                poly_count: 0,
            },
        }
    }

    /// Get polygon count
    pub fn polygon_count(&self) -> usize {
        self.polygons.len()
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Add a vertex
    pub fn add_vertex(&mut self, vertex: NavMeshVertex) -> u16 {
        let idx = self.vertices.len() as u16;
        self.vertices.push(vertex);
        self.header.vert_count = self.vertices.len() as u32;
        idx
    }

    /// Add a polygon
    pub fn add_polygon(&mut self, mut poly: NavMeshPoly) -> u32 {
        let index = self.polygons.len();
        if poly.poly_ref == 0 {
            poly.poly_ref = (index + 1) as u32;
        }
        self.poly_ref_map.insert(poly.poly_ref, index);
        self.polygons.push(poly);
        self.header.poly_count = self.polygons.len() as u32;
        self.polygons[index].poly_ref
    }

    /// Find nearest polygon to a position
    pub fn find_nearest_poly(&self, pos: Position) -> Option<usize> {
        let mut nearest_idx: Option<usize> = None;
        let mut nearest_dist = f32::MAX;

        for (idx, _poly) in self.polygons.iter().enumerate() {
            let center = self.get_poly_center(idx);
            let dx = pos.x - center.x;
            let dy = pos.y - center.y;
            let dz = pos.z - center.z;
            let dist_sq = dx * dx + dy * dy + dz * dz;

            if dist_sq < nearest_dist {
                nearest_dist = dist_sq;
                nearest_idx = Some(idx);
            }
        }

        nearest_idx
    }

    /// Find path using A* algorithm through polygon graph
    pub fn find_path(&self, start: Position, end: Position) -> PathResult {
        if self.polygons.is_empty() {
            return PathResult::NoPath;
        }

        let start_poly = self.find_nearest_poly(start);
        let end_poly = self.find_nearest_poly(end);

        let (Some(start_idx), Some(end_idx)) = (start_poly, end_poly) else {
            return PathResult::NoPath;
        };

        if start_idx == end_idx {
            return PathResult::StraightLine(start, end);
        }

        self.astar_pathfinding(start_idx, end_idx, start, end)
    }

    /// A* pathfinding between polygon indices
    fn astar_pathfinding(
        &self,
        start_idx: usize,
        end_idx: usize,
        start: Position,
        end: Position,
    ) -> PathResult {
        use pathfinding::prelude::astar;

        let end_center = self.get_poly_center(end_idx);

        let result = astar(
            &start_idx,
            |&idx| {
                // Get neighbors and costs inline
                let poly = &self.polygons[idx];
                let from_center = self.get_poly_center(idx);

                poly.neighbors
                    .iter()
                    .filter_map(|&neighbor_idx| {
                        if neighbor_idx == INVALID_POLY {
                            return None;
                        }
                        let neighbor_idx = neighbor_idx as usize;
                        if neighbor_idx >= self.polygons.len() {
                            return None;
                        }
                        let to_center = self.get_poly_center(neighbor_idx);
                        let dx = to_center.x - from_center.x;
                        let dy = to_center.y - from_center.y;
                        let dz = to_center.z - from_center.z;
                        Some((neighbor_idx, Cost((dx * dx + dy * dy + dz * dz).sqrt())))
                    })
                    .collect::<Vec<_>>()
            },
            |&idx| {
                let center = self.get_poly_center(idx);
                let dx = end_center.x - center.x;
                let dy = end_center.y - center.y;
                let dz = end_center.z - center.z;
                Cost((dx * dx + dy * dy + dz * dz).sqrt())
            },
            |&idx| idx == end_idx,
        );

        match result {
            Some((poly_path, _cost)) => {
                let waypoints = self.polygons_to_waypoints(&poly_path, start, end);
                if self.path_reaches_destination(&waypoints, end) {
                    PathResult::Complete(waypoints)
                } else {
                    PathResult::Partial(waypoints)
                }
            }
            None => PathResult::NoPath,
        }
    }

    /// Get polygon center point
    fn get_poly_center(&self, poly_idx: usize) -> NavMeshVertex {
        let poly = &self.polygons[poly_idx];
        let mut center = NavMeshVertex {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };

        if poly.vertex_indices.is_empty() {
            return center;
        }

        for &vertex_idx in &poly.vertex_indices {
            if let Some(vertex) = self.vertices.get(vertex_idx as usize) {
                center.x += vertex.x;
                center.y += vertex.y;
                center.z += vertex.z;
            }
        }

        let count = poly.vertex_indices.len() as f32;
        center.x /= count;
        center.y /= count;
        center.z /= count;

        center
    }

    /// Convert polygon path to waypoint path
    fn polygons_to_waypoints(
        &self,
        poly_path: &[usize],
        start: Position,
        end: Position,
    ) -> Vec<Position> {
        let mut waypoints = vec![start];

        for &poly_idx in poly_path.iter().skip(1) {
            let center = self.get_poly_center(poly_idx);
            waypoints.push(Position {
                x: center.x,
                y: center.y,
                z: center.z,
                o: 0.0,
            });
        }

        waypoints.push(end);
        waypoints
    }

    fn path_reaches_destination(&self, waypoints: &[Position], dest: Position) -> bool {
        if let Some(last) = waypoints.last() {
            let dx = last.x - dest.x;
            let dy = last.y - dest.y;
            let dz = last.z - dest.z;
            let dist_sq = dx * dx + dy * dy + dz * dz;
            dist_sq < 1.0
        } else {
            false
        }
    }

    /// Get a polygon by its poly_ref
    pub fn get_polygon(&self, poly_ref: u32) -> Option<&NavMeshPoly> {
        self.poly_ref_map
            .get(&poly_ref)
            .and_then(|&idx| self.polygons.get(idx))
    }

    /// Get a vertex by index
    pub fn get_vertex(&self, idx: u16) -> Option<&NavMeshVertex> {
        self.vertices.get(idx as usize)
    }

    /// Merge another navmesh's data into this one (for tile loading).
    /// Adjusts vertex indices and poly refs by current counts.
    pub fn merge_tile(&mut self, tile: &NavMesh) {
        // Check for overflow before casting
        if self.vertices.len() > u16::MAX as usize {
            tracing::warn!(
                "NavMesh has too many vertices ({}) to merge more tiles, skipping",
                self.vertices.len()
            );
            return;
        }

        let vertex_offset = self.vertices.len() as u16;
        let poly_ref_offset = self.polygons.len() as u32;

        // Add vertices from tile
        for i in 0..tile.vertex_count() {
            if let Some(vertex) = tile.get_vertex(i as u16) {
                self.add_vertex(*vertex);
            }
        }

        // Add polygons from tile (adjusting vertex indices and poly refs)
        for i in 0..tile.polygon_count() {
            let poly_ref = (i + 1) as u32;
            if let Some(tile_poly) = tile.get_polygon(poly_ref) {
                let mut poly = tile_poly.clone();

                // Adjust vertex indices by offset (with overflow check)
                for vert_idx in &mut poly.vertex_indices {
                    if let Some(new_idx) = vert_idx.checked_add(vertex_offset) {
                        *vert_idx = new_idx;
                    } else {
                        tracing::warn!(
                            "Vertex index overflow when merging navmesh tile, skipping polygon"
                        );
                        continue;
                    }
                }

                // Adjust polygon reference (with overflow check)
                if let Some(new_ref) = poly.poly_ref.checked_add(poly_ref_offset) {
                    poly.poly_ref = new_ref;
                } else {
                    tracing::warn!(
                        "Polygon reference overflow when merging navmesh tile, skipping polygon"
                    );
                    continue;
                }

                // Adjust neighbor references (with overflow check)
                let mut skip_polygon = false;
                for nei in &mut poly.neighbors {
                    if *nei != INVALID_POLY {
                        // Check if poly_ref_offset fits in u16
                        if poly_ref_offset > u16::MAX as u32 {
                            tracing::warn!(
                                "Polygon reference offset {} too large for u16, skipping polygon",
                                poly_ref_offset
                            );
                            skip_polygon = true;
                            break;
                        }

                        if let Some(new_nei) = nei.checked_add(poly_ref_offset as u16) {
                            *nei = new_nei;
                        } else {
                            tracing::warn!(
                                "Neighbor reference overflow when merging navmesh tile, skipping polygon"
                            );
                            skip_polygon = true;
                            break;
                        }
                    }
                }

                if skip_polygon {
                    continue;
                }

                self.add_polygon(poly);
            }
        }
    }
}

/// Parse Detour tile data into a NavMesh.
///
/// Parses the Detour binary tile format and extracts vertices and polygons.
/// The tile format:
/// - dtMeshHeader (64 bytes: counts and offsets)
/// - Vertices array (3 floats per vertex)
/// - Polygons array (dtPoly structures)
pub fn parse_detour_tile(tile_data: &[u8]) -> Option<NavMesh> {
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::Cursor;

    if tile_data.len() < 64 {
        return None;
    }

    let mut cursor = Cursor::new(tile_data);
    let mut navmesh = NavMesh::new();

    // Skip to vertex/polygon counts at offset 28
    cursor.set_position(28);

    let vert_count = cursor.read_u32::<LittleEndian>().ok()?;
    let poly_count = cursor.read_u32::<LittleEndian>().ok()?;

    let header_size: usize = 64;
    let vertex_offset = header_size;
    let vertex_size = (vert_count as usize) * 12; // 3 floats * 4 bytes

    // Read vertices
    cursor.set_position(vertex_offset as u64);

    for _ in 0..vert_count {
        let x = cursor.read_f32::<LittleEndian>().ok()?;
        let y = cursor.read_f32::<LittleEndian>().ok()?;
        let z = cursor.read_f32::<LittleEndian>().ok()?;

        navmesh.add_vertex(NavMeshVertex { x, y, z });
    }

    // Read polygons
    let poly_offset = vertex_offset + vertex_size;
    cursor.set_position(poly_offset as u64);

    const VERTS_PER_POLY: usize = 6; // Detour default
    let mut poly_ref_counter = 1u32;

    for _ in 0..poly_count {
        // Skip firstLink (4 bytes)
        if cursor.read_u32::<LittleEndian>().is_err() {
            break;
        }

        // Read vertex indices (up to VERTS_PER_POLY)
        let mut vertices = Vec::new();
        for _ in 0..VERTS_PER_POLY {
            let vert_idx = match cursor.read_u16::<LittleEndian>() {
                Ok(v) => v,
                Err(_) => break,
            };
            if vert_idx != 0xFFFF {
                vertices.push(vert_idx);
            }
        }

        // Read neighbor polygon refs
        let mut neighbors = Vec::new();
        for _ in 0..VERTS_PER_POLY {
            let nei_ref = match cursor.read_u16::<LittleEndian>() {
                Ok(v) => v,
                Err(_) => break,
            };
            if nei_ref != INVALID_POLY {
                neighbors.push(nei_ref);
            } else {
                neighbors.push(INVALID_POLY);
            }
        }

        // Read flags and counts
        let flags = match cursor.read_u16::<LittleEndian>() {
            Ok(v) => v,
            Err(_) => break,
        };
        let _vert_count = match cursor.read_u8() {
            Ok(v) => v,
            Err(_) => break,
        };
        let _area_and_type = match cursor.read_u8() {
            Ok(_) => (),
            Err(_) => break,
        };

        if !vertices.is_empty() {
            let poly = NavMeshPoly {
                vertex_indices: vertices,
                neighbors,
                flags,
                area: 0,
                poly_ref: poly_ref_counter,
            };
            navmesh.add_polygon(poly);
            poly_ref_counter += 1;
        }
    }

    if navmesh.polygon_count() == 0 {
        return None;
    }

    Some(navmesh)
}
