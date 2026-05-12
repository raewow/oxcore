//! Model Placement Extraction from ADT Files
//!
//! Extracts WMO and M2 model placement data from ADT files,
//! including position, rotation, scale, and unique IDs.

use anyhow::Result;
use glam::Vec3;
use std::collections::HashMap;

use crate::shared::formats::adt::ADTFile;
use crate::vmaps::transform::decode_scale;
use crate::vmaps::types::{BoundingBox, DoodadPlacement, WMOPlacement};

/// WMO Placement with filename
#[derive(Debug, Clone)]
pub struct WMOPlacementInfo {
    pub filename: String,
    pub placement: WMOPlacement,
}

/// M2 Doodad Placement with filename
#[derive(Debug, Clone)]
pub struct DoodadPlacementInfo {
    pub filename: String,
    pub placement: DoodadPlacement,
}

/// Extract WMO placements from ADT file
pub fn extract_wmo_placements(
    adt: &ADTFile,
    _map_id: u32,
    _tile_x: u32,
    _tile_y: u32,
) -> Result<Vec<WMOPlacementInfo>> {
    let mut placements = Vec::new();

    if let Some(ref modf) = adt.modf {
        for placement in &modf.placements {
            // Get filename from name list
            let filename = if (placement.name_id as usize) < adt.wmo_names.len() {
                adt.wmo_names[placement.name_id as usize].clone()
            } else {
                continue; // Skip invalid name ID
            };

            // Convert ADT placement to VMap placement
            let vmap_placement = WMOPlacement {
                unique_id: placement.unique_id,
                position: Vec3::from_array(placement.position),
                rotation: Vec3::from_array(placement.rotation),
                bounding_box: BoundingBox::new(
                    Vec3::from_array(placement.bounding_box_min),
                    Vec3::from_array(placement.bounding_box_max),
                ),
                flags: placement.flags,
                doodad_set: placement.doodad_set,
                name_set: placement.name_set,
                scale: placement.scale,
            };

            placements.push(WMOPlacementInfo {
                filename,
                placement: vmap_placement,
            });
        }
    }

    Ok(placements)
}

/// Extract M2 doodad placements from ADT file
pub fn extract_doodad_placements(
    adt: &ADTFile,
    _map_id: u32,
    _tile_x: u32,
    _tile_y: u32,
) -> Result<Vec<DoodadPlacementInfo>> {
    let mut placements = Vec::new();

    if let Some(ref mddf) = adt.mddf {
        for placement in &mddf.placements {
            // Get filename from name list
            let filename = if (placement.name_id as usize) < adt.model_names.len() {
                adt.model_names[placement.name_id as usize].clone()
            } else {
                continue; // Skip invalid name ID
            };

            // Convert ADT placement to VMap placement
            let vmap_placement = DoodadPlacement {
                name_index: placement.name_id,
                unique_id: placement.unique_id,
                position: Vec3::from_array(placement.position),
                rotation: Vec3::from_array(placement.rotation),
                scale: placement.scale,
                flags: placement.flags,
            };

            placements.push(DoodadPlacementInfo {
                filename,
                placement: vmap_placement,
            });
        }
    }

    Ok(placements)
}

/// Generate unique object ID for model instances
///
/// Uses a map to ensure the same (client_id, name_index) pair always gets the same ID
pub fn generate_unique_object_id(
    client_id: u32,
    name_index: u32,
    id_map: &mut HashMap<(u32, u32), u32>,
) -> u32 {
    let key = (client_id, name_index);

    // Check if key exists first
    if let Some(&id) = id_map.get(&key) {
        return id;
    }

    // Generate new ID
    let new_id = (id_map.len() + 1) as u32;
    id_map.insert(key, new_id);
    new_id
}

/// Get world position from tile coordinates and local position
pub fn get_world_position(tile_x: u32, tile_y: u32, local_pos: Vec3) -> Vec3 {
    // WoW coordinate system: each tile is 533.33 yards
    // Tiles are numbered from (0,0) in the northwest to (63,63) in southeast
    // The map center is at tile (32,32)
    const TILE_SIZE: f32 = 533.33333;
    const MAP_SIZE: f32 = TILE_SIZE * 64.0; // 64x64 tiles = 34133.33 yards
    const MAP_HALF: f32 = MAP_SIZE / 2.0;   // 17066.67 yards

    // Calculate world position
    // X axis: tile 0 = northwest (positive), tile 63 = southeast (negative)
    // Y axis: tile 0 = northwest (positive), tile 63 = southeast (negative)
    let world_x = MAP_HALF - (tile_x as f32 * TILE_SIZE) - local_pos.x;
    let world_y = MAP_HALF - (tile_y as f32 * TILE_SIZE) - local_pos.y;

    Vec3::new(world_x, world_y, local_pos.z)
}

/// Extract scale as float from u16 format
pub fn extract_scale(scale_u16: u16) -> f32 {
    decode_scale(scale_u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unique_object_id() {
        let mut map = HashMap::new();

        let id1 = generate_unique_object_id(100, 1, &mut map);
        let id2 = generate_unique_object_id(100, 1, &mut map);
        let id3 = generate_unique_object_id(100, 2, &mut map);
        let id4 = generate_unique_object_id(200, 1, &mut map);

        // Same input should give same ID
        assert_eq!(id1, id2);

        // Different inputs should give different IDs
        assert_ne!(id1, id3);
        assert_ne!(id1, id4);
        assert_ne!(id3, id4);

        // IDs should be sequential
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn test_get_world_position_center() {
        // Tile (32, 32) is the center tile
        let pos = get_world_position(32, 32, Vec3::ZERO);

        // Should be near world center (may not be exactly 0,0,0 due to tile offset)
        assert!(pos.x.abs() < 300.0);
        assert!(pos.y.abs() < 300.0);
        assert_eq!(pos.z, 0.0);
    }

    #[test]
    fn test_get_world_position_northwest() {
        // Tile (0, 0) is northwest corner
        let pos = get_world_position(0, 0, Vec3::ZERO);

        // Should be in positive X, positive Y quadrant
        assert!(pos.x > 0.0);
        assert!(pos.y > 0.0);
    }

    #[test]
    fn test_get_world_position_southeast() {
        // Tile (63, 63) is southeast corner
        let pos = get_world_position(63, 63, Vec3::ZERO);

        // Should be in negative X, negative Y quadrant
        assert!(pos.x < 0.0);
        assert!(pos.y < 0.0);
    }

    #[test]
    fn test_extract_scale() {
        assert_eq!(extract_scale(1024), 1.0);
        assert_eq!(extract_scale(2048), 2.0);
        assert_eq!(extract_scale(512), 0.5);
        assert_eq!(extract_scale(0), 0.0);
    }

    // Note: Integration tests with actual ADT files are better done in integration tests
    // due to ADTFile private fields. These unit tests cover the core logic.
}
