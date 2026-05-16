//! MMap manager - loads and manages navigation meshes
//!
//! Loads .mmap header files and .mmtile tile files, parses Detour binary format
//! into Rust-native NavMesh polygons, and provides A* pathfinding.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use super::navmesh::{parse_detour_tile, NavMesh};
use super::types::PathResult;
use super::vmap::VMapManager;
use crate::shared::protocol::Position;
use tracing::{debug, info, warn};

/// MMap file magic and version (must match extractor output)
const MMAP_MAGIC: u32 = 0x4d4d4150; // "MMAP"
const MMAP_VERSION: u32 = 6;

/// Pack tile coordinates into a single ID
fn pack_tile_id(x: i32, y: i32) -> u32 {
    ((x as u32) << 16) | (y as u32 & 0x0000FFFF)
}

/// Per-map MMap data: navmesh + loaded tile tracking
struct MMapData {
    /// Rust-native navmesh (populated by merging parsed .mmtile data)
    navmesh: NavMesh,
    /// Loaded tiles: packed(x,y) -> raw tile data (for tracking what's loaded)
    loaded_tiles: HashMap<u32, Vec<u8>>,
}

impl MMapData {
    fn new() -> Self {
        Self {
            navmesh: NavMesh::new(),
            loaded_tiles: HashMap::new(),
        }
    }
}

/// MMap manager with integrated VMap
pub struct MMapManager {
    /// Per-map data (map_id -> MMapData)
    map_data: RwLock<HashMap<u32, MMapData>>,
    /// Data directory path (parent of mmaps/)
    data_dir: std::path::PathBuf,
    /// Path to mmaps/ directory
    mmaps_dir: std::path::PathBuf,
    /// VMap manager for collision (integrated)
    vmap: Arc<VMapManager>,
    /// Whether mmaps directory exists
    loaded: bool,
    /// Total loaded tile count
    loaded_tile_count: RwLock<u32>,
}

impl MMapManager {
    pub fn new(data_dir: impl Into<std::path::PathBuf>, vmap: Arc<VMapManager>) -> Self {
        let data_dir = data_dir.into();
        let mmaps_dir = data_dir.join("mmaps");
        let loaded = mmaps_dir.exists();

        if loaded {
            info!("MMapManager: mmaps directory found at {:?}", mmaps_dir);
        } else {
            warn!(
                "MMapManager: mmaps directory not found at {:?}, pathfinding will use fallback",
                mmaps_dir
            );
        }

        Self {
            map_data: RwLock::new(HashMap::new()),
            data_dir,
            mmaps_dir,
            vmap,
            loaded,
            loaded_tile_count: RwLock::new(0),
        }
    }

    /// Check if navmesh is loaded for a map
    pub fn has_navmesh(&self, map_id: u32) -> bool {
        self.map_data.read().contains_key(&map_id)
    }

    /// Load map header (.mmap file) - creates empty navmesh container for tiles
    pub fn load_map_data(&self, map_id: u32) -> bool {
        // Already loaded?
        if self.map_data.read().contains_key(&map_id) {
            return true;
        }

        if !self.loaded {
            return false;
        }

        let path = self.mmaps_dir.join(format!("{:03}.mmap", map_id));
        if !path.exists() {
            debug!("MMAP: No navmesh file for map {} at {:?}", map_id, path);
            return false;
        }

        // Read and validate .mmap header (dtNavMeshParams: origin[3], tileWidth, tileHeight, maxTiles, maxPolys = 28 bytes)
        match std::fs::File::open(&path) {
            Ok(mut file) => {
                use byteorder::{LittleEndian, ReadBytesExt};
                // Read origin (3 floats)
                let ox = file.read_f32::<LittleEndian>().unwrap_or(0.0);
                let oy = file.read_f32::<LittleEndian>().unwrap_or(0.0);
                let oz = file.read_f32::<LittleEndian>().unwrap_or(0.0);
                let tile_w = file.read_f32::<LittleEndian>().unwrap_or(0.0);
                let tile_h = file.read_f32::<LittleEndian>().unwrap_or(0.0);
                let max_tiles = file.read_i32::<LittleEndian>().unwrap_or(0);
                let max_polys = file.read_i32::<LittleEndian>().unwrap_or(0);

                debug!(
                    "MMAP: Loaded {:03}.mmap (origin: [{:.1}, {:.1}, {:.1}], tile: {:.0}x{:.0}, maxTiles: {}, maxPolys: {})",
                    map_id, ox, oy, oz, tile_w, tile_h, max_tiles, max_polys
                );

                // Create empty MMapData - tiles will be loaded separately
                self.map_data.write().insert(map_id, MMapData::new());
                true
            }
            Err(e) => {
                warn!("MMAP: Failed to open {:03}.mmap: {}", map_id, e);
                false
            }
        }
    }

    /// Load a map tile (.mmtile file) and merge into navmesh
    ///
    /// File format: `mmaps/{map_id:03}{y:02}{x:02}.mmtile`
    /// Header: magic(u32) + dtVersion(u32) + mmapVersion(u32) + size(u32) + usesLiquids(u32)
    /// Body: raw Detour tile data (size bytes)
    pub fn load_map_tile(&self, map_id: u32, x: i32, y: i32) -> bool {
        // Ensure map header is loaded
        if !self.load_map_data(map_id) {
            return false;
        }

        let packed = pack_tile_id(x, y);

        // Already loaded?
        {
            let data = self.map_data.read();
            if let Some(mmap) = data.get(&map_id) {
                if mmap.loaded_tiles.contains_key(&packed) {
                    return true;
                }
            }
        }

        // Format matches C++/extractor: mmaps/{mapId:03}{y:02}{x:02}.mmtile
        let filename = format!("{:03}{:02}{:02}.mmtile", map_id, y, x);
        let path = self.mmaps_dir.join(&filename);

        if !path.exists() {
            // Normal - many tiles don't have navmesh data
            return false;
        }

        // Read and parse .mmtile file
        let tile_data = match self.read_mmtile(&path, map_id, x, y) {
            Some(data) => data,
            None => return false,
        };

        // Parse Detour binary into NavMesh polygons
        let tile_navmesh = match parse_detour_tile(&tile_data) {
            Some(nm) => nm,
            None => {
                warn!("MMAP: Failed to parse tile {}", filename);
                return false;
            }
        };

        // Merge tile navmesh into map's main navmesh
        let mut data = self.map_data.write();
        if let Some(mmap) = data.get_mut(&map_id) {
            let poly_before = mmap.navmesh.polygon_count();
            mmap.navmesh.merge_tile(&tile_navmesh);
            let poly_after = mmap.navmesh.polygon_count();

            mmap.loaded_tiles.insert(packed, tile_data);
            *self.loaded_tile_count.write() += 1;

            debug!(
                "MMAP: Loaded tile {} (+{} polys, total {} polys)",
                filename,
                poly_after - poly_before,
                poly_after
            );
            true
        } else {
            false
        }
    }

    /// Read and validate a .mmtile file, returning raw Detour tile data
    fn read_mmtile(&self, path: &std::path::Path, map_id: u32, x: i32, y: i32) -> Option<Vec<u8>> {
        use byteorder::{LittleEndian, ReadBytesExt};
        use std::io::Read;

        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                debug!(
                    "MMAP: Could not open {:03}{:02}{:02}.mmtile: {}",
                    map_id, y, x, e
                );
                return None;
            }
        };

        // Read MmapTileHeader: magic(u32) + dtVersion(u32) + mmapVersion(u32) + size(u32) + usesLiquids(u32)
        let mut magic_bytes = [0u8; 4];
        if file.read_exact(&mut magic_bytes).is_err() {
            return None;
        }
        let magic = u32::from_le_bytes(magic_bytes);

        if magic != MMAP_MAGIC {
            warn!(
                "MMAP: Bad header magic in {:03}{:02}{:02}.mmtile",
                map_id, y, x
            );
            return None;
        }

        let _dt_version = file.read_u32::<LittleEndian>().ok()?;
        let version = file.read_u32::<LittleEndian>().ok()?;

        if version != MMAP_VERSION {
            warn!(
                "MMAP: {:03}{:02}{:02}.mmtile version {} != expected {}",
                map_id, y, x, version, MMAP_VERSION
            );
            return None;
        }

        let tile_size = file.read_u32::<LittleEndian>().ok()?;
        let _uses_liquids = file.read_u32::<LittleEndian>().ok()?;

        // Read raw Detour tile data
        let mut data = vec![0u8; tile_size as usize];
        if file.read_exact(&mut data).is_err() {
            warn!(
                "MMAP: Truncated tile data in {:03}{:02}{:02}.mmtile",
                map_id, y, x
            );
            return None;
        }

        Some(data)
    }

    /// Unload a single tile
    pub fn unload_tile(&self, map_id: u32, x: i32, y: i32) {
        let packed = pack_tile_id(x, y);
        let mut data = self.map_data.write();
        if let Some(mmap) = data.get_mut(&map_id) {
            if mmap.loaded_tiles.remove(&packed).is_some() {
                *self.loaded_tile_count.write() -= 1;
                // Note: Cannot efficiently remove individual tile polygons from merged navmesh.
                // For tile unloading, we rebuild from remaining tiles.
                self.rebuild_navmesh_from_tiles(mmap);
                debug!("MMAP: Unloaded tile {:03}{:02}{:02}.mmtile", map_id, y, x);
            }
        }
    }

    /// Unload all data for a map
    pub fn unload_map(&self, map_id: u32) {
        let mut data = self.map_data.write();
        if let Some(mmap) = data.remove(&map_id) {
            let count = mmap.loaded_tiles.len() as u32;
            *self.loaded_tile_count.write() -= count;
            info!("MMAP: Unloaded map {} ({} tiles)", map_id, count);
        }
    }

    /// Rebuild navmesh from remaining loaded tiles (after tile removal)
    fn rebuild_navmesh_from_tiles(&self, mmap: &mut MMapData) {
        let mut new_navmesh = NavMesh::new();
        for tile_data in mmap.loaded_tiles.values() {
            if let Some(tile_nm) = parse_detour_tile(tile_data) {
                new_navmesh.merge_tile(&tile_nm);
            }
        }
        mmap.navmesh = new_navmesh;
    }

    /// Calculate path using navmesh A* algorithm
    pub fn calculate_path(&self, map_id: u32, start: Position, end: Position) -> PathResult {
        let data = self.map_data.read();
        let Some(mmap) = data.get(&map_id) else {
            return PathResult::NoPath;
        };

        // Only use navmesh path if it has polygons loaded
        if mmap.navmesh.polygon_count() == 0 {
            return PathResult::NoPath;
        }

        mmap.navmesh.find_path(start, end)
    }

    /// Get VMap manager reference
    pub fn vmap(&self) -> &Arc<VMapManager> {
        &self.vmap
    }

    /// Get the data directory
    pub fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }

    /// Check if mmaps directory is available
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Get total loaded tile count across all maps
    pub fn loaded_tile_count(&self) -> u32 {
        *self.loaded_tile_count.read()
    }

    /// Get polygon count for a map's navmesh
    pub fn polygon_count(&self, map_id: u32) -> usize {
        self.map_data
            .read()
            .get(&map_id)
            .map(|m| m.navmesh.polygon_count())
            .unwrap_or(0)
    }
}
