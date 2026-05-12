//! MMap (Movement Map) Navigation Mesh Generation
//!
//! Generates navigation meshes for pathfinding using Recast/Detour.
//! Output is compatible with MaNGOS server expectations.

mod terrain_builder;
mod tile_builder;
mod file_writer;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::{debug, info, warn};

use terrain_builder::{MeshData, TerrainBuilder};
use tile_builder::TileBuilder;
use file_writer::{GridBounds, MMapWriter, NavMeshParams};

/// Generate navigation meshes for all maps
pub fn generate(input: &Path, output: &Path, filter: Vec<u32>, debug_meshes: bool) -> Result<()> {
    info!("Generating navigation meshes...");

    let maps_dir = output.join("maps");
    let vmaps_dir = output.join("vmaps");

    if !maps_dir.exists() {
        warn!("Maps directory not found: {}. Run 'maps' extraction first.", maps_dir.display());
        return Ok(());
    }

    // Create mmap writer
    let writer = MMapWriter::new(output)?;

    // Discover available tiles from map files
    let tiles = discover_tiles(&maps_dir, &filter)?;

    if tiles.is_empty() {
        warn!("No map tiles found to process");
        return Ok(());
    }

    info!("Found {} maps with tiles to process", tiles.len());

    // Process each map
    for (map_id, tile_coords) in &tiles {
        build_map(*map_id, tile_coords, &maps_dir, &vmaps_dir, &writer, debug_meshes)?;
    }

    info!("Navigation mesh generation complete");

    Ok(())
}

/// Discover available tiles from map files
fn discover_tiles(maps_dir: &Path, filter: &[u32]) -> Result<HashMap<u32, Vec<(u32, u32)>>> {
    let mut tiles: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();

    // Scan maps directory for .map files
    for entry in fs::read_dir(maps_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "map") {
            if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                // Parse filename: {mapId:03}{y:02}{x:02}.map
                if filename.len() == 7 {
                    if let (Ok(map_id), Ok(tile_y), Ok(tile_x)) = (
                        filename[0..3].parse::<u32>(),
                        filename[3..5].parse::<u32>(),
                        filename[5..7].parse::<u32>(),
                    ) {
                        // Apply filter if specified
                        if filter.is_empty() || filter.contains(&map_id) {
                            tiles.entry(map_id).or_default().push((tile_x, tile_y));
                        }
                    }
                }
            }
        }
    }

    // Sort tiles for consistent processing
    for coords in tiles.values_mut() {
        coords.sort();
    }

    Ok(tiles)
}

/// Build navigation meshes for a single map
fn build_map(
    map_id: u32,
    tile_coords: &[(u32, u32)],
    maps_dir: &Path,
    vmaps_dir: &Path,
    writer: &MMapWriter,
    debug_meshes: bool,
) -> Result<()> {
    info!("[Map {:03}] Building {} tiles...", map_id, tile_coords.len());

    // Calculate grid bounds
    let mut bounds = GridBounds::new();
    for &(x, y) in tile_coords {
        bounds.extend(x, y);
    }

    if !bounds.is_valid() {
        warn!("[Map {:03}] No valid tiles found", map_id);
        return Ok(());
    }

    // Write map header
    let params = NavMeshParams::for_map(map_id, &bounds);
    writer.write_map_header(map_id, &params)?;

    // Progress bar
    let progress = ProgressBar::new(tile_coords.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[Map {msg}] [{bar:40.cyan/blue}] {pos}/{len} tiles")
            .unwrap()
            .progress_chars("=>-"),
    );
    progress.set_message(format!("{:03}", map_id));

    // Count of successfully built tiles
    let built_count = AtomicU32::new(0);
    let skip_count = AtomicU32::new(0);

    // Process tiles (can be parallelized with rayon)
    // For now, process sequentially to manage memory and debug output
    for &(tile_x, tile_y) in tile_coords {
        // Skip if tile already exists
        if writer.tile_exists(map_id, tile_x, tile_y) {
            skip_count.fetch_add(1, Ordering::Relaxed);
            progress.inc(1);
            continue;
        }

        // Build tile
        match build_tile(map_id, tile_x, tile_y, maps_dir, vmaps_dir, writer, debug_meshes) {
            Ok(true) => {
                built_count.fetch_add(1, Ordering::Relaxed);
            }
            Ok(false) => {
                // Empty tile, no data
            }
            Err(e) => {
                warn!(
                    "[Map {:03}] Failed to build tile [{},{}]: {}",
                    map_id, tile_x, tile_y, e
                );
            }
        }

        progress.inc(1);
    }

    progress.finish_and_clear();

    let built = built_count.load(Ordering::Relaxed);
    let skipped = skip_count.load(Ordering::Relaxed);

    info!(
        "[Map {:03}] Complete: {} tiles built, {} skipped (already exist)",
        map_id, built, skipped
    );

    Ok(())
}

/// Build a single navigation mesh tile
fn build_tile(
    map_id: u32,
    tile_x: u32,
    tile_y: u32,
    maps_dir: &Path,
    vmaps_dir: &Path,
    writer: &MMapWriter,
    debug_meshes: bool,
) -> Result<bool> {
    debug!(
        "[Map {:03}] Building tile [{},{}]",
        map_id, tile_x, tile_y
    );

    // Create terrain builder
    let mut terrain_builder = TerrainBuilder::new(false);
    let mut mesh_data = MeshData::default();

    // Load terrain data
    let has_terrain = terrain_builder.load_map(maps_dir, map_id, tile_x, tile_y, &mut mesh_data)?;

    if !has_terrain {
        return Ok(false);
    }

    // TODO: Load VMap collision geometry
    // This would load WMO/M2 models from the vmaps directory and add them to mesh_data
    // load_vmap(vmaps_dir, map_id, tile_x, tile_y, &mut mesh_data)?;

    // Clean up unused vertices
    TerrainBuilder::clean_vertices(&mut mesh_data.solid_verts, &mut mesh_data.solid_tris);
    TerrainBuilder::clean_vertices(&mut mesh_data.liquid_verts, &mut mesh_data.liquid_tris);

    // Check if we have any geometry
    if mesh_data.solid_verts.is_empty() && mesh_data.liquid_verts.is_empty() {
        return Ok(false);
    }

    // Build navigation mesh tile
    let tile_builder = TileBuilder::new(map_id);
    let tile_data = tile_builder.build_tile(map_id, tile_x, tile_y, &mesh_data)?;

    // Write tile data
    if let Some(data) = tile_data {
        if !data.data.is_empty() {
            writer.write_tile(map_id, tile_x, tile_y, &data)?;
            return Ok(true);
        }
    }

    // Even without actual navmesh data, we processed the tile
    Ok(false)
}

/// Load VMap collision geometry (WMO/M2 models)
#[allow(dead_code)]
fn load_vmap(
    vmaps_dir: &Path,
    map_id: u32,
    tile_x: u32,
    tile_y: u32,
    mesh_data: &mut MeshData,
) -> Result<()> {
    // TODO: Implement VMap loading
    // This would:
    // 1. Load .vmtree file for the map
    // 2. Load .vmtile files for referenced models
    // 3. Extract collision geometry and add to mesh_data

    // For now, just check if vmap files exist
    let vmtree_file = vmaps_dir.join(format!("{:03}.vmtree", map_id));
    if !vmtree_file.exists() {
        debug!(
            "[Map {:03}] VMap tree file not found: {}",
            map_id,
            vmtree_file.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_discover_tiles_empty() {
        let temp_dir = TempDir::new().unwrap();
        let tiles = discover_tiles(temp_dir.path(), &[]).unwrap();
        assert!(tiles.is_empty());
    }

    #[test]
    fn test_discover_tiles_with_filter() {
        let temp_dir = TempDir::new().unwrap();

        // Create some mock map files
        fs::write(temp_dir.path().join("0003232.map"), b"test").unwrap();
        fs::write(temp_dir.path().join("0013333.map"), b"test").unwrap();
        fs::write(temp_dir.path().join("4493434.map"), b"test").unwrap();

        // Filter to only map 0
        let tiles = discover_tiles(temp_dir.path(), &[0]).unwrap();
        assert_eq!(tiles.len(), 1);
        assert!(tiles.contains_key(&0));
        assert_eq!(tiles[&0], vec![(32, 32)]);

        // Filter to map 0 and 1
        let tiles = discover_tiles(temp_dir.path(), &[0, 1]).unwrap();
        assert_eq!(tiles.len(), 2);

        // No filter (all maps)
        let tiles = discover_tiles(temp_dir.path(), &[]).unwrap();
        assert_eq!(tiles.len(), 3);
    }
}
