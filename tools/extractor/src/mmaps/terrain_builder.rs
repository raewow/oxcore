//! Terrain Builder for MMap Generation
//!
//! Loads terrain data from extracted .map files and converts it to
//! mesh geometry for Recast navmesh generation.

use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use tracing::debug;

// Map file constants (must match map extractor output)
const MAP_MAGIC: &[u8; 4] = b"MAPS";
const MAP_VERSION_MAGIC: &[u8; 4] = b"z1.4";
const MAP_AREA_MAGIC: &[u8; 4] = b"AREA";
const MAP_HEIGHT_MAGIC: &[u8; 4] = b"MHGT";
const MAP_LIQUID_MAGIC: &[u8; 4] = b"MLIQ";

// Grid constants from MaNGOS
pub const GRID_SIZE: f32 = 533.33333;
pub const GRID_PART_SIZE: f32 = GRID_SIZE / 128.0;
pub const MAP_RESOLUTION: usize = 128;
pub const V9_SIZE: usize = 129;
pub const V8_SIZE: usize = 128;

// Height flags
const MAP_HEIGHT_NO_HEIGHT: u32 = 0x0001;
const MAP_HEIGHT_AS_INT16: u32 = 0x0002;
const MAP_HEIGHT_AS_INT8: u32 = 0x0004;

// Liquid flags
const MAP_LIQUID_NO_TYPE: u8 = 0x01;
const MAP_LIQUID_NO_HEIGHT: u8 = 0x02;

// Liquid types
pub const MAP_LIQUID_TYPE_NO_WATER: u8 = 0x00;
pub const MAP_LIQUID_TYPE_MAGMA: u8 = 0x01;
pub const MAP_LIQUID_TYPE_OCEAN: u8 = 0x02;
pub const MAP_LIQUID_TYPE_SLIME: u8 = 0x04;
pub const MAP_LIQUID_TYPE_WATER: u8 = 0x08;

// Invalid height marker
pub const INVALID_MAP_LIQ_HEIGHT: f32 = -500.0;
pub const INVALID_MAP_LIQ_HEIGHT_MAX: f32 = 5000.0;

/// Mesh data for Recast input
#[derive(Debug, Default)]
pub struct MeshData {
    /// Solid terrain vertices (y, z, x format for Recast)
    pub solid_verts: Vec<f32>,
    /// Solid terrain triangle indices
    pub solid_tris: Vec<i32>,
    /// Liquid vertices
    pub liquid_verts: Vec<f32>,
    /// Liquid triangle indices
    pub liquid_tris: Vec<i32>,
    /// Liquid area types (per triangle)
    pub liquid_type: Vec<u8>,
}

/// Terrain builder for loading map data
pub struct TerrainBuilder {
    skip_liquid: bool,
    v9: Box<[[f32; V9_SIZE]; V9_SIZE]>,
    v8: Box<[[f32; V8_SIZE]; V8_SIZE]>,
}

impl TerrainBuilder {
    pub fn new(skip_liquid: bool) -> Self {
        Self {
            skip_liquid,
            v9: Box::new([[0.0; V9_SIZE]; V9_SIZE]),
            v8: Box::new([[0.0; V8_SIZE]; V8_SIZE]),
        }
    }

    /// Load terrain data for a map tile
    pub fn load_map(
        &mut self,
        maps_dir: &Path,
        map_id: u32,
        tile_x: u32,
        tile_y: u32,
        mesh_data: &mut MeshData,
    ) -> Result<bool> {
        let filename = format!("{:03}{:02}{:02}.map", map_id, tile_y, tile_x);
        let map_path = maps_dir.join(&filename);

        if !map_path.exists() {
            debug!("Map file not found: {}", map_path.display());
            return Ok(false);
        }

        // Load height map
        self.load_height_map(&map_path)?;

        // Generate terrain mesh
        self.generate_terrain_mesh(tile_x, tile_y, mesh_data)?;

        // Load liquid if enabled
        if !self.skip_liquid {
            self.load_liquid_map(&map_path, tile_x, tile_y, mesh_data)?;
        }

        Ok(true)
    }

    /// Load height data from .map file
    fn load_height_map(&mut self, map_path: &Path) -> Result<()> {
        let mut file = File::open(map_path)
            .with_context(|| format!("Failed to open map file: {}", map_path.display()))?;

        // Read and verify header
        let mut header = [0u8; 40];
        file.read_exact(&mut header)?;

        if &header[0..4] != MAP_MAGIC || &header[4..8] != MAP_VERSION_MAGIC {
            anyhow::bail!("Invalid map file format: {}", map_path.display());
        }

        // Parse header offsets
        let height_map_offset = u32::from_le_bytes([header[16], header[17], header[18], header[19]]);

        if height_map_offset == 0 {
            // No height data - fill with zeros
            for y in 0..V9_SIZE {
                for x in 0..V9_SIZE {
                    self.v9[y][x] = 0.0;
                }
            }
            for y in 0..V8_SIZE {
                for x in 0..V8_SIZE {
                    self.v8[y][x] = 0.0;
                }
            }
            return Ok(());
        }

        // Seek to height data
        file.seek(SeekFrom::Start(height_map_offset as u64))?;

        // Read height header
        let mut fourcc = [0u8; 4];
        file.read_exact(&mut fourcc)?;
        if &fourcc != MAP_HEIGHT_MAGIC {
            anyhow::bail!("Invalid height map magic");
        }

        let flags = file.read_u32::<LittleEndian>()?;
        let grid_height = file.read_f32::<LittleEndian>()?;
        let _grid_max_height = file.read_f32::<LittleEndian>()?;

        if flags & MAP_HEIGHT_NO_HEIGHT != 0 {
            // Flat terrain - use grid_height for all
            for y in 0..V9_SIZE {
                for x in 0..V9_SIZE {
                    self.v9[y][x] = grid_height;
                }
            }
            for y in 0..V8_SIZE {
                for x in 0..V8_SIZE {
                    self.v8[y][x] = grid_height;
                }
            }
            return Ok(());
        }

        // Read height data based on format
        if flags & MAP_HEIGHT_AS_INT8 != 0 {
            self.read_height_int8(&mut file, grid_height)?;
        } else if flags & MAP_HEIGHT_AS_INT16 != 0 {
            self.read_height_int16(&mut file, grid_height)?;
        } else {
            self.read_height_float(&mut file)?;
        }

        Ok(())
    }

    fn read_height_float(&mut self, file: &mut File) -> Result<()> {
        // Read V9 (129x129)
        for y in 0..V9_SIZE {
            for x in 0..V9_SIZE {
                self.v9[y][x] = file.read_f32::<LittleEndian>()?;
            }
        }
        // Read V8 (128x128)
        for y in 0..V8_SIZE {
            for x in 0..V8_SIZE {
                self.v8[y][x] = file.read_f32::<LittleEndian>()?;
            }
        }
        Ok(())
    }

    fn read_height_int16(&mut self, file: &mut File, base_height: f32) -> Result<()> {
        // Read V9
        for y in 0..V9_SIZE {
            for x in 0..V9_SIZE {
                let val = file.read_u16::<LittleEndian>()? as f32;
                self.v9[y][x] = base_height + val / 65535.0 * 2048.0;
            }
        }
        // Read V8
        for y in 0..V8_SIZE {
            for x in 0..V8_SIZE {
                let val = file.read_u16::<LittleEndian>()? as f32;
                self.v8[y][x] = base_height + val / 65535.0 * 2048.0;
            }
        }
        Ok(())
    }

    fn read_height_int8(&mut self, file: &mut File, base_height: f32) -> Result<()> {
        // Read V9
        for y in 0..V9_SIZE {
            for x in 0..V9_SIZE {
                let val = file.read_u8()? as f32;
                self.v9[y][x] = base_height + val / 255.0 * 2.0;
            }
        }
        // Read V8
        for y in 0..V8_SIZE {
            for x in 0..V8_SIZE {
                let val = file.read_u8()? as f32;
                self.v8[y][x] = base_height + val / 255.0 * 2.0;
            }
        }
        Ok(())
    }

    /// Generate terrain mesh from height data
    fn generate_terrain_mesh(
        &self,
        tile_x: u32,
        tile_y: u32,
        mesh_data: &mut MeshData,
    ) -> Result<()> {
        // Calculate world offset for this tile
        let x_offset = (32.0 - tile_x as f32) * GRID_SIZE;
        let y_offset = (32.0 - tile_y as f32) * GRID_SIZE;

        // Generate vertices and triangles for each cell
        for y in 0..V8_SIZE {
            for x in 0..V8_SIZE {
                let base_vert = mesh_data.solid_verts.len() / 3;

                // Calculate world positions
                let px = x_offset - (x as f32 * GRID_PART_SIZE);
                let py = y_offset - (y as f32 * GRID_PART_SIZE);

                // Get heights at corners and center
                let h_tl = self.v9[y][x];
                let h_tr = self.v9[y][x + 1];
                let h_bl = self.v9[y + 1][x];
                let h_br = self.v9[y + 1][x + 1];
                let h_center = self.v8[y][x];

                // Add 5 vertices per cell (4 corners + center)
                // Recast uses (x, y, z) where y is up
                // WoW uses (x, y, z) where z is up
                // We convert: WoW(x,y,z) -> Recast(y, z, x)

                // Top-left
                mesh_data.solid_verts.push(py);
                mesh_data.solid_verts.push(h_tl);
                mesh_data.solid_verts.push(px);

                // Top-right
                mesh_data.solid_verts.push(py);
                mesh_data.solid_verts.push(h_tr);
                mesh_data.solid_verts.push(px - GRID_PART_SIZE);

                // Bottom-left
                mesh_data.solid_verts.push(py - GRID_PART_SIZE);
                mesh_data.solid_verts.push(h_bl);
                mesh_data.solid_verts.push(px);

                // Bottom-right
                mesh_data.solid_verts.push(py - GRID_PART_SIZE);
                mesh_data.solid_verts.push(h_br);
                mesh_data.solid_verts.push(px - GRID_PART_SIZE);

                // Center
                mesh_data.solid_verts.push(py - GRID_PART_SIZE / 2.0);
                mesh_data.solid_verts.push(h_center);
                mesh_data.solid_verts.push(px - GRID_PART_SIZE / 2.0);

                // Add 4 triangles (fan from center)
                let tl = base_vert as i32;
                let tr = (base_vert + 1) as i32;
                let bl = (base_vert + 2) as i32;
                let br = (base_vert + 3) as i32;
                let center = (base_vert + 4) as i32;

                // Top triangle
                mesh_data.solid_tris.push(tl);
                mesh_data.solid_tris.push(tr);
                mesh_data.solid_tris.push(center);

                // Right triangle
                mesh_data.solid_tris.push(tr);
                mesh_data.solid_tris.push(br);
                mesh_data.solid_tris.push(center);

                // Bottom triangle
                mesh_data.solid_tris.push(br);
                mesh_data.solid_tris.push(bl);
                mesh_data.solid_tris.push(center);

                // Left triangle
                mesh_data.solid_tris.push(bl);
                mesh_data.solid_tris.push(tl);
                mesh_data.solid_tris.push(center);
            }
        }

        Ok(())
    }

    /// Load liquid data from .map file
    fn load_liquid_map(
        &self,
        map_path: &Path,
        tile_x: u32,
        tile_y: u32,
        mesh_data: &mut MeshData,
    ) -> Result<()> {
        let mut file = File::open(map_path)?;

        // Read header
        let mut header = [0u8; 40];
        file.read_exact(&mut header)?;

        let liquid_map_offset =
            u32::from_le_bytes([header[24], header[25], header[26], header[27]]);

        if liquid_map_offset == 0 {
            return Ok(());
        }

        // Seek to liquid data
        file.seek(SeekFrom::Start(liquid_map_offset as u64))?;

        // Read liquid header
        let mut fourcc = [0u8; 4];
        file.read_exact(&mut fourcc)?;
        if &fourcc != MAP_LIQUID_MAGIC {
            return Ok(());
        }

        let flags = file.read_u8()?;
        let liquid_flags_global = file.read_u8()?;
        let liquid_type_global = file.read_u16::<LittleEndian>()?;
        let offset_x = file.read_u8()? as usize;
        let offset_y = file.read_u8()? as usize;
        let width = file.read_u8()? as usize;
        let height = file.read_u8()? as usize;
        let liquid_level = file.read_f32::<LittleEndian>()?;

        if width == 0 || height == 0 {
            return Ok(());
        }

        // Read liquid type data if present
        let mut liquid_entry = [[0u16; 16]; 16];
        let mut liquid_flags = [[0u8; 16]; 16];

        if flags & MAP_LIQUID_NO_TYPE == 0 {
            for y in 0..16 {
                for x in 0..16 {
                    liquid_entry[y][x] = file.read_u16::<LittleEndian>()?;
                }
            }
            for y in 0..16 {
                for x in 0..16 {
                    liquid_flags[y][x] = file.read_u8()?;
                }
            }
        } else {
            for y in 0..16 {
                for x in 0..16 {
                    liquid_entry[y][x] = liquid_type_global;
                    liquid_flags[y][x] = liquid_flags_global;
                }
            }
        }

        // Read liquid heights if present
        let mut liquid_height = vec![vec![liquid_level; width]; height];
        if flags & MAP_LIQUID_NO_HEIGHT == 0 {
            for y in 0..height {
                for x in 0..width {
                    liquid_height[y][x] = file.read_f32::<LittleEndian>()?;
                }
            }
        }

        // Generate liquid mesh
        let x_offset = (32.0 - tile_x as f32) * GRID_SIZE;
        let y_offset = (32.0 - tile_y as f32) * GRID_SIZE;

        for y in 0..(height - 1) {
            for x in 0..(width - 1) {
                let cell_x = (offset_x + x) / 8;
                let cell_y = (offset_y + y) / 8;

                if cell_x >= 16 || cell_y >= 16 {
                    continue;
                }

                let liq_flags = liquid_flags[cell_y][cell_x];
                if liq_flags == 0 {
                    continue;
                }

                // Determine liquid type for area marking
                let area_type = if liq_flags & MAP_LIQUID_TYPE_WATER != 0 {
                    6 // AREA_WATER
                } else if liq_flags & MAP_LIQUID_TYPE_MAGMA != 0 {
                    7 // AREA_MAGMA
                } else if liq_flags & MAP_LIQUID_TYPE_SLIME != 0 {
                    8 // AREA_SLIME
                } else {
                    6 // Default to water
                };

                let base_vert = mesh_data.liquid_verts.len() / 3;

                // World positions
                let px = x_offset - ((offset_x + x) as f32 * GRID_PART_SIZE);
                let py = y_offset - ((offset_y + y) as f32 * GRID_PART_SIZE);

                // Heights
                let h_tl = liquid_height[y][x];
                let h_tr = liquid_height[y][x + 1];
                let h_bl = liquid_height[y + 1][x];
                let h_br = liquid_height[y + 1][x + 1];

                // Add 4 vertices (quad)
                mesh_data.liquid_verts.push(py);
                mesh_data.liquid_verts.push(h_tl);
                mesh_data.liquid_verts.push(px);

                mesh_data.liquid_verts.push(py);
                mesh_data.liquid_verts.push(h_tr);
                mesh_data.liquid_verts.push(px - GRID_PART_SIZE);

                mesh_data.liquid_verts.push(py - GRID_PART_SIZE);
                mesh_data.liquid_verts.push(h_bl);
                mesh_data.liquid_verts.push(px);

                mesh_data.liquid_verts.push(py - GRID_PART_SIZE);
                mesh_data.liquid_verts.push(h_br);
                mesh_data.liquid_verts.push(px - GRID_PART_SIZE);

                // Add 2 triangles
                let tl = base_vert as i32;
                let tr = (base_vert + 1) as i32;
                let bl = (base_vert + 2) as i32;
                let br = (base_vert + 3) as i32;

                mesh_data.liquid_tris.push(tl);
                mesh_data.liquid_tris.push(tr);
                mesh_data.liquid_tris.push(bl);
                mesh_data.liquid_type.push(area_type);

                mesh_data.liquid_tris.push(tr);
                mesh_data.liquid_tris.push(br);
                mesh_data.liquid_tris.push(bl);
                mesh_data.liquid_type.push(area_type);
            }
        }

        Ok(())
    }

    /// Get height at a specific position
    pub fn get_height(&self, x: f32, y: f32) -> f32 {
        // Convert to grid coordinates
        let gx = ((x / GRID_PART_SIZE) + 0.5) as usize;
        let gy = ((y / GRID_PART_SIZE) + 0.5) as usize;

        if gx < V8_SIZE && gy < V8_SIZE {
            self.v8[gy][gx]
        } else {
            0.0
        }
    }

    /// Clean up unused vertices from mesh data
    pub fn clean_vertices(verts: &mut Vec<f32>, tris: &mut Vec<i32>) {
        if verts.is_empty() || tris.is_empty() {
            return;
        }

        // Track which vertices are used
        let vert_count = verts.len() / 3;
        let mut used = vec![false; vert_count];

        for &idx in tris.iter() {
            if (idx as usize) < vert_count {
                used[idx as usize] = true;
            }
        }

        // Build remap table
        let mut remap = vec![0i32; vert_count];
        let mut new_idx = 0i32;
        for i in 0..vert_count {
            if used[i] {
                remap[i] = new_idx;
                new_idx += 1;
            }
        }

        // Compact vertices
        let mut new_verts = Vec::with_capacity((new_idx as usize) * 3);
        for i in 0..vert_count {
            if used[i] {
                new_verts.push(verts[i * 3]);
                new_verts.push(verts[i * 3 + 1]);
                new_verts.push(verts[i * 3 + 2]);
            }
        }

        // Remap triangle indices
        for idx in tris.iter_mut() {
            *idx = remap[*idx as usize];
        }

        *verts = new_verts;
    }
}
