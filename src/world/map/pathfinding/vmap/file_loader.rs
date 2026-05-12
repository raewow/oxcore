//! VMap file loading (.vmtree, .vmtile, .vmo)
//! Aligned with MaNGOS VMapManager2 and MapTree implementation

use crate::shared::protocol::Position;
use super::types::{BoundingBox, ModelInstance, ModelType};
use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// VMap file magic string
const VMAP_MAGIC: &[u8] = b"VMAP_7.0";
const VMAP_VERSION: u32 = 7;

/// Model flags (from MaNGOS ModelInstance.h)
const MOD_HAS_BOUND: u32 = 1 << 2;

/// VMap file loader
pub struct VMapFileLoader {
    base_path: PathBuf,
}

impl VMapFileLoader {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Load map directory file (.vmtree)
    pub fn load_map_tree(&self, map_id: u32) -> Result<MapTreeData> {
        let filename = format!("{:03}.vmtree", map_id);
        let path = self.base_path.join(&filename);

        if !path.exists() {
            return Err(anyhow::anyhow!(
                "VMap tree file not found: {}",
                path.display()
            ));
        }

        let mut file = BufReader::new(
            File::open(&path)
                .with_context(|| format!("Failed to open VMap tree file: {}", path.display()))?,
        );

        // Read magic (8 bytes: "VMAP_7.0")
        let mut magic = vec![0u8; VMAP_MAGIC.len()];
        file.read_exact(&mut magic)?;

        if magic != VMAP_MAGIC {
            return Err(anyhow::anyhow!(
                "Invalid VMap magic in file: {}",
                path.display()
            ));
        }

        // Read tiled flag (1 byte char)
        let tiled = file.read_u8()?;
        let is_tiled = tiled != 0;

        // Read "NODE" chunk (4 bytes)
        let mut node_chunk = [0u8; 4];
        file.read_exact(&mut node_chunk)?;
        if &node_chunk != b"NODE" {
            return Err(anyhow::anyhow!(
                "Expected NODE chunk, got: {:?}",
                std::str::from_utf8(&node_chunk)
            ));
        }

        // Read BIH tree data
        // lo (3 floats) - bounding box min
        let _lo_x = file.read_f32::<LittleEndian>()?;
        let _lo_y = file.read_f32::<LittleEndian>()?;
        let _lo_z = file.read_f32::<LittleEndian>()?;

        // hi (3 floats) - bounding box max
        let _hi_x = file.read_f32::<LittleEndian>()?;
        let _hi_y = file.read_f32::<LittleEndian>()?;
        let _hi_z = file.read_f32::<LittleEndian>()?;

        // treeSize (uint32)
        let tree_size = file.read_u32::<LittleEndian>()?;

        const MAX_TREE_SIZE: u32 = 10_000_000;
        if tree_size > MAX_TREE_SIZE {
            return Err(anyhow::anyhow!(
                "VMap tree size too large: {} (max: {}) - file may be corrupted",
                tree_size,
                MAX_TREE_SIZE
            ));
        }

        // tree array (uint32[treeSize])
        let mut tree = vec![0u32; tree_size as usize];
        for i in 0..tree_size {
            tree[i as usize] = file.read_u32::<LittleEndian>()?;
        }

        // count (uint32) - number of objects
        let count = file.read_u32::<LittleEndian>()?;

        const MAX_OBJECT_COUNT: u32 = 10_000_000;
        if count > MAX_OBJECT_COUNT {
            return Err(anyhow::anyhow!(
                "VMap object count too large: {} (max: {}) - file may be corrupted",
                count,
                MAX_OBJECT_COUNT
            ));
        }

        // objects array (uint32[count])
        let mut objects = vec![0u32; count as usize];
        for i in 0..count {
            objects[i as usize] = file.read_u32::<LittleEndian>()?;
        }

        // Read "GOBJ" chunk (4 bytes)
        let mut gobj_chunk = [0u8; 4];
        file.read_exact(&mut gobj_chunk)?;
        if &gobj_chunk != b"GOBJ" {
            return Err(anyhow::anyhow!(
                "Expected GOBJ chunk, got: {:?}",
                std::str::from_utf8(&gobj_chunk)
            ));
        }

        let tiles = Vec::new();
        let tile_count = 0u32;

        if !is_tiled {
            let max_spawns = 10000;
            let mut spawn_count = 0;
            loop {
                if spawn_count >= max_spawns {
                    warn!(
                        "VMap tree file {} has too many ModelSpawns (>{}) - stopping",
                        path.display(),
                        max_spawns
                    );
                    break;
                }

                match self.read_model_spawn(&mut file) {
                    Ok(Some(_spawn)) => {
                        spawn_count += 1;
                        match file.read_u32::<LittleEndian>() {
                            Ok(_referenced_val) => {}
                            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                            Err(_) => break,
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }

            if spawn_count > 0 {
                debug!("Loaded {} ModelSpawns from VMap tree file", spawn_count);
            }
        }

        info!(
            "Loaded VMap tree: map {} (tiled: {}, tree_size: {}, objects: {})",
            map_id, is_tiled, tree_size, count
        );

        Ok(MapTreeData {
            map_id,
            tile_count,
            tiles,
            is_tiled,
            tree_size,
            object_count: count,
        })
    }

    /// Read ModelSpawn from file
    fn read_model_spawn(&self, file: &mut BufReader<File>) -> Result<Option<ModelSpawn>> {
        let flags = match file.read_u32::<LittleEndian>() {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let adt_id = file.read_u16::<LittleEndian>()?;
        let id = file.read_u32::<LittleEndian>()?;

        let pos_x = file.read_f32::<LittleEndian>()?;
        let pos_y = file.read_f32::<LittleEndian>()?;
        let pos_z = file.read_f32::<LittleEndian>()?;

        let rot_x = file.read_f32::<LittleEndian>()?;
        let rot_y = file.read_f32::<LittleEndian>()?;
        let rot_z = file.read_f32::<LittleEndian>()?;

        let scale = file.read_f32::<LittleEndian>()?;

        let has_bound = (flags & MOD_HAS_BOUND) != 0;
        if has_bound {
            let _b_low_x = file.read_f32::<LittleEndian>()?;
            let _b_low_y = file.read_f32::<LittleEndian>()?;
            let _b_low_z = file.read_f32::<LittleEndian>()?;
            let _b_high_x = file.read_f32::<LittleEndian>()?;
            let _b_high_y = file.read_f32::<LittleEndian>()?;
            let _b_high_z = file.read_f32::<LittleEndian>()?;
        }

        let name_len = file.read_u32::<LittleEndian>()?;
        if name_len > 500 {
            return Err(anyhow::anyhow!("ModelSpawn name too long: {}", name_len));
        }

        let mut name_buf = vec![0u8; name_len as usize];
        file.read_exact(&mut name_buf)?;
        let name = String::from_utf8(name_buf)
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in ModelSpawn name: {}", e))?;

        Ok(Some(ModelSpawn {
            flags,
            adt_id,
            id,
            pos: (pos_x, pos_y, pos_z),
            rot: (rot_x, rot_y, rot_z),
            scale,
            name,
        }))
    }

    /// Load map tile file (.vmtile)
    /// Returns Ok(None) if tile file doesn't exist (normal)
    pub fn load_map_tile(&self, map_id: u32, x: u32, y: u32) -> Result<Option<MapTileData>> {
        let filename = format!("{:03}_{:02}_{:02}.vmtile", map_id, x, y);
        let path = self.base_path.join(&filename);

        if !path.exists() {
            return Ok(None);
        }

        let mut file = BufReader::new(
            File::open(&path)
                .with_context(|| format!("Failed to open VMap tile file: {}", path.display()))?,
        );

        // Read magic
        let mut magic = vec![0u8; VMAP_MAGIC.len()];
        file.read_exact(&mut magic)?;

        if magic != VMAP_MAGIC {
            return Err(anyhow::anyhow!(
                "Invalid VMap magic in tile file: {}",
                path.display()
            ));
        }

        // Read numSpawns
        let num_spawns = file.read_u32::<LittleEndian>()?;

        const MAX_TILE_SPAWNS: u32 = 100_000;
        if num_spawns > MAX_TILE_SPAWNS {
            return Err(anyhow::anyhow!(
                "VMap tile numSpawns too large: {} (max: {})",
                num_spawns,
                MAX_TILE_SPAWNS
            ));
        }

        // Read model spawns
        let mut model_instances = Vec::new();
        for _i in 0..num_spawns {
            let spawn = match self.read_model_spawn(&mut file)? {
                Some(s) => s,
                None => break,
            };

            let _referenced_val = file.read_u32::<LittleEndian>()?;

            let position = Position::new(spawn.pos.0, spawn.pos.1, spawn.pos.2, 0.0);
            let rotation = [spawn.rot.0, spawn.rot.1, spawn.rot.2];

            let model_type = if spawn.name.contains(".wmo") || spawn.name.contains("buildings/") {
                ModelType::WMO
            } else {
                ModelType::M2
            };

            model_instances.push(ModelInstance {
                model_id: spawn.id,
                model_type,
                position,
                scale: spawn.scale,
                rotation,
                model_name: spawn.name.clone(),
            });
        }

        debug!(
            "Loaded VMap tile: {} ({}, {}) - {} models",
            map_id, x, y, model_instances.len()
        );

        Ok(Some(MapTileData {
            map_id,
            tile_x: x,
            tile_y: y,
            model_instances,
        }))
    }

    /// Load world model object file (.vmo) by model name
    /// Reads VMAP_7.0 chunk-based format (WMOD/GMOD/VERT/TRIM/MBIH/LIQU/GBIH)
    pub fn load_world_model(&self, model_name: &str) -> Result<WorldModelData> {
        // .vmo files are in the vmaps dir, named <model_name>.vmo
        let vmo_name = format!("{}.vmo", model_name);
        let path = self.base_path.join(&vmo_name);

        if !path.exists() {
            return Err(anyhow::anyhow!(
                "World model file not found: {}",
                path.display()
            ));
        }

        let mut file = BufReader::new(
            File::open(&path)
                .with_context(|| format!("Failed to open world model file: {}", path.display()))?,
        );

        // Read magic (8 bytes: "VMAP_7.0")
        let mut magic = vec![0u8; VMAP_MAGIC.len()];
        file.read_exact(&mut magic)?;

        if magic != VMAP_MAGIC {
            return Err(anyhow::anyhow!(
                "Invalid VMap magic in model file: {}",
                path.display()
            ));
        }

        // Read "WMOD" chunk
        let mut chunk = [0u8; 4];
        file.read_exact(&mut chunk)?;
        if &chunk != b"WMOD" {
            return Err(anyhow::anyhow!(
                "Expected WMOD chunk in {}",
                path.display()
            ));
        }
        let _chunk_size = file.read_u32::<LittleEndian>()?;
        let _root_wmo_id = file.read_u32::<LittleEndian>()?;

        // Try to read "GMOD" chunk (may not exist for empty models)
        let mut groups = Vec::new();
        match file.read_exact(&mut chunk) {
            Ok(()) if &chunk == b"GMOD" => {
                let group_count = file.read_u32::<LittleEndian>()?;

                for _ in 0..group_count {
                    match self.read_group_model(&mut file) {
                        Ok(group) => groups.push(group),
                        Err(e) => {
                            warn!("Failed to read group in {}: {}", path.display(), e);
                            break;
                        }
                    }
                }

                // Skip "GBIH" chunk (group-level BIH tree - not needed for BSP approach)
                let _ = file.read_exact(&mut chunk); // "GBIH"
                // We don't need to read the BIH data since we build our own BSP tree
            }
            _ => {
                // No GMOD chunk - empty model
            }
        }

        debug!(
            "Loaded world model: {} ({} groups)",
            model_name, groups.len()
        );

        Ok(WorldModelData { model_name: model_name.to_string(), groups })
    }

    /// Read a single group model from chunk-based .vmo format
    fn read_group_model(&self, file: &mut BufReader<File>) -> Result<GroupModel> {
        // Read bounding box (AABox = 6 floats)
        let min_x = file.read_f32::<LittleEndian>()?;
        let min_y = file.read_f32::<LittleEndian>()?;
        let min_z = file.read_f32::<LittleEndian>()?;
        let max_x = file.read_f32::<LittleEndian>()?;
        let max_y = file.read_f32::<LittleEndian>()?;
        let max_z = file.read_f32::<LittleEndian>()?;

        let bounding_box = BoundingBox {
            min: Position::new(min_x, min_y, min_z, 0.0),
            max: Position::new(max_x, max_y, max_z, 0.0),
        };

        // Read mogpFlags
        let _mogp_flags = file.read_u32::<LittleEndian>()?;

        // Read GroupWMOID
        let _group_wmo_id = file.read_u32::<LittleEndian>()?;

        // Read "VERT" chunk
        let mut chunk = [0u8; 4];
        file.read_exact(&mut chunk)?;
        if &chunk != b"VERT" {
            return Err(anyhow::anyhow!("Expected VERT chunk, got {:?}", std::str::from_utf8(&chunk)));
        }
        let _chunk_size = file.read_u32::<LittleEndian>()?;
        let vert_count = file.read_u32::<LittleEndian>()?;

        if vert_count == 0 {
            // Models without geometry end here (matches MaNGOS early return)
            return Ok(GroupModel {
                bounding_box,
                triangles: Vec::new(),
                liquid_data: None,
            });
        }

        let mut vertices = Vec::with_capacity(vert_count as usize);
        for _ in 0..vert_count {
            let x = file.read_f32::<LittleEndian>()?;
            let y = file.read_f32::<LittleEndian>()?;
            let z = file.read_f32::<LittleEndian>()?;
            vertices.push(Position::new(x, y, z, 0.0));
        }

        // Read "TRIM" chunk (triangle mesh)
        file.read_exact(&mut chunk)?;
        if &chunk != b"TRIM" {
            return Err(anyhow::anyhow!("Expected TRIM chunk, got {:?}", std::str::from_utf8(&chunk)));
        }
        let _chunk_size = file.read_u32::<LittleEndian>()?;
        let tri_count = file.read_u32::<LittleEndian>()?;

        let mut triangles = Vec::with_capacity(tri_count as usize);
        for _ in 0..tri_count {
            let i0 = file.read_u32::<LittleEndian>()? as usize;
            let i1 = file.read_u32::<LittleEndian>()? as usize;
            let i2 = file.read_u32::<LittleEndian>()? as usize;

            if i0 < vertices.len() && i1 < vertices.len() && i2 < vertices.len() {
                triangles.push(Triangle {
                    v0: vertices[i0],
                    v1: vertices[i1],
                    v2: vertices[i2],
                });
            }
        }

        // Read "MBIH" chunk (per-group mesh BIH tree - skip it, we build our own BSP)
        file.read_exact(&mut chunk)?;
        if &chunk == b"MBIH" {
            // Skip BIH data: bounds(6f) + treeSize(u32) + tree[treeSize] + count(u32) + objects[count]
            let _lo_x = file.read_f32::<LittleEndian>()?;
            let _lo_y = file.read_f32::<LittleEndian>()?;
            let _lo_z = file.read_f32::<LittleEndian>()?;
            let _hi_x = file.read_f32::<LittleEndian>()?;
            let _hi_y = file.read_f32::<LittleEndian>()?;
            let _hi_z = file.read_f32::<LittleEndian>()?;
            let tree_size = file.read_u32::<LittleEndian>()?;
            for _ in 0..tree_size {
                let _ = file.read_u32::<LittleEndian>()?;
            }
            let obj_count = file.read_u32::<LittleEndian>()?;
            for _ in 0..obj_count {
                let _ = file.read_u32::<LittleEndian>()?;
            }
        }

        // Read "LIQU" chunk
        file.read_exact(&mut chunk)?;
        let liquid_data = if &chunk == b"LIQU" {
            let chunk_size = file.read_u32::<LittleEndian>()?;
            if chunk_size > 0 {
                // Read WmoLiquid: tilesX(u32), tilesY(u32), corner(3f), type(u32), heights, flags
                let tiles_x = file.read_u32::<LittleEndian>()?;
                let tiles_y = file.read_u32::<LittleEndian>()?;
                let _corner_x = file.read_f32::<LittleEndian>()?;
                let _corner_y = file.read_f32::<LittleEndian>()?;
                let corner_z = file.read_f32::<LittleEndian>()?;
                let liquid_type = file.read_u32::<LittleEndian>()?;

                // Read height array
                let height_count = ((tiles_x + 1) * (tiles_y + 1)) as usize;
                let mut max_level = f32::NEG_INFINITY;
                for _ in 0..height_count {
                    let h = file.read_f32::<LittleEndian>()?;
                    if h > max_level {
                        max_level = h;
                    }
                }

                // Read flags array
                let flag_count = (tiles_x * tiles_y) as usize;
                let mut flags_buf = vec![0u8; flag_count];
                file.read_exact(&mut flags_buf)?;

                Some(LiquidData {
                    liquid_type,
                    level: max_level,
                    floor: corner_z,
                })
            } else {
                None
            }
        } else {
            None
        };

        Ok(GroupModel {
            bounding_box,
            triangles,
            liquid_data,
        })
    }
}

/// ModelSpawn structure (internal)
#[derive(Debug, Clone)]
struct ModelSpawn {
    flags: u32,
    adt_id: u16,
    id: u32,
    pos: (f32, f32, f32),
    rot: (f32, f32, f32),
    scale: f32,
    name: String,
}

/// Map tree data (from .vmtree file)
#[derive(Debug, Clone)]
pub struct MapTreeData {
    pub map_id: u32,
    pub tile_count: u32,
    pub tiles: Vec<(u32, u32)>,
    pub is_tiled: bool,
    pub tree_size: u32,
    pub object_count: u32,
}

/// Map tile data (from .vmtile file)
#[derive(Debug, Clone)]
pub struct MapTileData {
    pub map_id: u32,
    pub tile_x: u32,
    pub tile_y: u32,
    pub model_instances: Vec<ModelInstance>,
}

/// World model data (from .vmo file)
#[derive(Debug, Clone)]
pub struct WorldModelData {
    pub model_name: String,
    pub groups: Vec<GroupModel>,
}

/// Group model (part of world model)
#[derive(Debug, Clone)]
pub struct GroupModel {
    pub bounding_box: BoundingBox,
    pub triangles: Vec<Triangle>,
    pub liquid_data: Option<LiquidData>,
}

/// Triangle (3 vertices)
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub v0: Position,
    pub v1: Position,
    pub v2: Position,
}

/// Liquid data
#[derive(Debug, Clone)]
pub struct LiquidData {
    pub liquid_type: u32,
    pub level: f32,
    pub floor: f32,
}
