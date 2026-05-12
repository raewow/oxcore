//! MMap File Writer
//!
//! Writes navigation mesh data in MaNGOS-compatible format.
//! Output files: .mmap (map header) and .mmtile (tile data)

use anyhow::{Context, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use super::terrain_builder::GRID_SIZE;
use super::tile_builder::NavMeshTileData;

// MMap file constants (must match server expectations)
const MMAP_MAGIC: u32 = 0x4d4d4150; // "MMAP" in little-endian
const MMAP_VERSION: u32 = 6;
const DT_NAVMESH_VERSION: u32 = 7; // Detour navmesh version

/// dtNavMeshParams structure for .mmap file
#[derive(Debug, Clone, Default)]
pub struct NavMeshParams {
    pub origin: [f32; 3],
    pub tile_width: f32,
    pub tile_height: f32,
    pub max_tiles: i32,
    pub max_polys: i32,
}

impl NavMeshParams {
    /// Create params for a standard map
    pub fn for_map(map_id: u32, grid_bounds: &GridBounds) -> Self {
        // Calculate based on tile coverage
        let tile_size = GRID_SIZE; // Each tile covers one grid

        // Origin at max corner (WoW coordinate system)
        let origin = [
            (32.0 - grid_bounds.min_x as f32) * GRID_SIZE,
            -500.0, // Min height
            (32.0 - grid_bounds.min_y as f32) * GRID_SIZE,
        ];

        // Calculate number of tiles needed
        let tiles_x = (grid_bounds.max_x - grid_bounds.min_x + 1) as i32;
        let tiles_y = (grid_bounds.max_y - grid_bounds.min_y + 1) as i32;
        let max_tiles = tiles_x * tiles_y;

        Self {
            origin,
            tile_width: tile_size,
            tile_height: tile_size,
            max_tiles: max_tiles.max(1),
            max_polys: 1 << 14, // 16384 polys per tile (default)
        }
    }

    /// Write to file
    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_f32::<LittleEndian>(self.origin[0])?;
        writer.write_f32::<LittleEndian>(self.origin[1])?;
        writer.write_f32::<LittleEndian>(self.origin[2])?;
        writer.write_f32::<LittleEndian>(self.tile_width)?;
        writer.write_f32::<LittleEndian>(self.tile_height)?;
        writer.write_i32::<LittleEndian>(self.max_tiles)?;
        writer.write_i32::<LittleEndian>(self.max_polys)?;
        Ok(())
    }
}

/// Grid bounds for a map
#[derive(Debug, Clone, Default)]
pub struct GridBounds {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

impl GridBounds {
    pub fn new() -> Self {
        Self {
            min_x: u32::MAX,
            min_y: u32::MAX,
            max_x: 0,
            max_y: 0,
        }
    }

    pub fn extend(&mut self, x: u32, y: u32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    pub fn is_valid(&self) -> bool {
        self.min_x <= self.max_x && self.min_y <= self.max_y
    }
}

/// MMap tile header (MmapTileHeader)
#[derive(Debug, Clone)]
pub struct MMapTileHeader {
    pub mmap_magic: u32,
    pub dt_version: u32,
    pub mmap_version: u32,
    pub size: u32,
    pub uses_liquids: u32,
}

impl MMapTileHeader {
    pub fn new(tile_data_size: u32, uses_liquids: bool) -> Self {
        Self {
            mmap_magic: MMAP_MAGIC,
            dt_version: DT_NAVMESH_VERSION,
            mmap_version: MMAP_VERSION,
            size: tile_data_size,
            uses_liquids: if uses_liquids { 1 } else { 0 },
        }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LittleEndian>(self.mmap_magic)?;
        writer.write_u32::<LittleEndian>(self.dt_version)?;
        writer.write_u32::<LittleEndian>(self.mmap_version)?;
        writer.write_u32::<LittleEndian>(self.size)?;
        writer.write_u32::<LittleEndian>(self.uses_liquids)?;
        Ok(())
    }
}

/// MMap file writer
pub struct MMapWriter {
    output_dir: std::path::PathBuf,
}

impl MMapWriter {
    pub fn new(output_dir: &Path) -> Result<Self> {
        // Create mmaps directory if it doesn't exist
        let mmaps_dir = output_dir.join("mmaps");
        fs::create_dir_all(&mmaps_dir)
            .with_context(|| format!("Failed to create mmaps directory: {}", mmaps_dir.display()))?;

        Ok(Self {
            output_dir: mmaps_dir,
        })
    }

    /// Write map header file (.mmap)
    pub fn write_map_header(&self, map_id: u32, params: &NavMeshParams) -> Result<()> {
        let filename = format!("{:03}.mmap", map_id);
        let path = self.output_dir.join(&filename);

        let file = File::create(&path)
            .with_context(|| format!("Failed to create mmap file: {}", path.display()))?;
        let mut writer = BufWriter::new(file);

        params.write(&mut writer)?;
        writer.flush()?;

        tracing::debug!("Wrote map header: {}", path.display());
        Ok(())
    }

    /// Write tile data file (.mmtile)
    pub fn write_tile(
        &self,
        map_id: u32,
        tile_x: u32,
        tile_y: u32,
        tile_data: &NavMeshTileData,
    ) -> Result<()> {
        if tile_data.data.is_empty() {
            tracing::debug!(
                "Skipping empty tile [{},{}] for map {}",
                tile_x,
                tile_y,
                map_id
            );
            return Ok(());
        }

        // Format: {mapId:03}{y:02}{x:02}.mmtile
        let filename = format!("{:03}{:02}{:02}.mmtile", map_id, tile_y, tile_x);
        let path = self.output_dir.join(&filename);

        let file = File::create(&path)
            .with_context(|| format!("Failed to create mmtile file: {}", path.display()))?;
        let mut writer = BufWriter::new(file);

        // Write header
        let header = MMapTileHeader::new(tile_data.data.len() as u32, tile_data.uses_liquids);
        header.write(&mut writer)?;

        // Write tile data
        writer.write_all(&tile_data.data)?;
        writer.flush()?;

        tracing::debug!(
            "Wrote tile [{},{}]: {} ({} bytes)",
            tile_x,
            tile_y,
            path.display(),
            tile_data.data.len()
        );
        Ok(())
    }

    /// Write GameObject navmesh file
    pub fn write_gameobject(&self, display_id: u32, tile_data: &NavMeshTileData) -> Result<()> {
        if tile_data.data.is_empty() {
            return Ok(());
        }

        // Format: go{displayId:04}.mmtile
        let filename = format!("go{:04}.mmtile", display_id);
        let path = self.output_dir.join(&filename);

        let file = File::create(&path)
            .with_context(|| format!("Failed to create GO mmtile file: {}", path.display()))?;
        let mut writer = BufWriter::new(file);

        let header = MMapTileHeader::new(tile_data.data.len() as u32, tile_data.uses_liquids);
        header.write(&mut writer)?;
        writer.write_all(&tile_data.data)?;
        writer.flush()?;

        tracing::debug!(
            "Wrote GameObject mmtile: {} ({} bytes)",
            path.display(),
            tile_data.data.len()
        );
        Ok(())
    }

    /// Check if a tile file already exists
    pub fn tile_exists(&self, map_id: u32, tile_x: u32, tile_y: u32) -> bool {
        let filename = format!("{:03}{:02}{:02}.mmtile", map_id, tile_y, tile_x);
        self.output_dir.join(&filename).exists()
    }

    /// Check if map header file exists
    pub fn map_header_exists(&self, map_id: u32) -> bool {
        let filename = format!("{:03}.mmap", map_id);
        self.output_dir.join(&filename).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_nav_mesh_params_write() {
        let params = NavMeshParams {
            origin: [1.0, 2.0, 3.0],
            tile_width: 533.33333,
            tile_height: 533.33333,
            max_tiles: 64,
            max_polys: 16384,
        };

        let mut buffer = Vec::new();
        params.write(&mut buffer).unwrap();

        // Should be 24 bytes: 3*f32 + f32 + f32 + i32 + i32
        assert_eq!(buffer.len(), 24);
    }

    #[test]
    fn test_mmap_tile_header_write() {
        let header = MMapTileHeader::new(1000, true);

        let mut buffer = Vec::new();
        header.write(&mut buffer).unwrap();

        // Should be 20 bytes: 5 * u32
        assert_eq!(buffer.len(), 20);

        // Verify magic
        let magic = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        assert_eq!(magic, MMAP_MAGIC);
    }

    #[test]
    fn test_mmap_writer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let writer = MMapWriter::new(temp_dir.path()).unwrap();

        // Verify mmaps directory was created
        assert!(temp_dir.path().join("mmaps").exists());
    }

    #[test]
    fn test_grid_bounds() {
        let mut bounds = GridBounds::new();
        assert!(!bounds.is_valid());

        bounds.extend(10, 20);
        bounds.extend(5, 15);
        bounds.extend(15, 25);

        assert!(bounds.is_valid());
        assert_eq!(bounds.min_x, 5);
        assert_eq!(bounds.min_y, 15);
        assert_eq!(bounds.max_x, 15);
        assert_eq!(bounds.max_y, 25);
    }
}
