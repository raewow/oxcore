//! VMap Object (.vmo) Converter
//!
//! Converts raw VMAPs05 model files (from extraction phase) into server-compatible
//! VMAP_7.0 .vmo files. This is the equivalent of MaNGOS TileAssembler::convertRawFile().
//!
//! Raw format (VMAPs05): Written by the extractor for WMO and M2 models
//! Final format (VMAP_7.0): Chunk-based format read by the server (WorldModel::readFile)

use anyhow::{Context, Result, bail};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use glam::Vec3;
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use std::fs::File;
use std::path::Path;
use tracing::{debug, warn};

use crate::vmaps::tree::bih::BIH;
use crate::vmaps::types::BoundingBox;

/// Magic for raw extraction files
const RAW_VMAP_MAGIC: &[u8; 8] = b"VMAPs05\0";

/// Magic for server-compatible .vmo files
const VMAP_MAGIC: &[u8; 8] = b"VMAP_7.0";

/// MeshTriangle - 3 vertex indices (u32 each, matching MaNGOS)
#[derive(Debug, Clone, Copy)]
struct MeshTriangle {
    idx0: u32,
    idx1: u32,
    idx2: u32,
}

/// Liquid data from raw format
#[derive(Debug, Clone)]
struct WmoLiquid {
    tiles_x: u32,
    tiles_y: u32,
    corner: Vec3,
    liquid_type: u32,
    heights: Vec<f32>,
    flags: Vec<u8>,
}

impl WmoLiquid {
    /// Write liquid data in server format
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LittleEndian>(self.tiles_x)?;
        writer.write_u32::<LittleEndian>(self.tiles_y)?;
        writer.write_f32::<LittleEndian>(self.corner.x)?;
        writer.write_f32::<LittleEndian>(self.corner.y)?;
        writer.write_f32::<LittleEndian>(self.corner.z)?;
        writer.write_u32::<LittleEndian>(self.liquid_type)?;
        for &h in &self.heights {
            writer.write_f32::<LittleEndian>(h)?;
        }
        for &f in &self.flags {
            writer.write_u8(f)?;
        }
        Ok(())
    }

    /// Get serialized size in bytes
    fn file_size(&self) -> u32 {
        // iTilesX(4) + iTilesY(4) + iCorner(12) + iType(4) + heights + flags
        4 + 4 + 12 + 4
            + ((self.tiles_x + 1) * (self.tiles_y + 1)) * 4
            + (self.tiles_x * self.tiles_y)
    }
}

/// Raw group data read from VMAPs05 format
#[derive(Debug, Clone)]
struct RawGroupModel {
    mogp_flags: u32,
    group_wmo_id: u32,
    bounds: BoundingBox,
    vertices: Vec<Vec3>,
    triangles: Vec<MeshTriangle>,
    liquid: Option<WmoLiquid>,
}

/// Raw world model read from VMAPs05 format
#[derive(Debug, Clone)]
struct RawWorldModel {
    root_wmo_id: u32,
    groups: Vec<RawGroupModel>,
}

/// Read a raw VMAPs05 model file (equivalent to MaNGOS WorldModel_Raw::Read)
fn read_raw_model(path: &Path) -> Result<RawWorldModel> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open raw model: {}", path.display()))?;
    let mut reader = BufReader::new(file);

    // Read and verify magic
    let mut magic = [0u8; 8];
    reader.read_exact(&mut magic)?;
    if &magic != RAW_VMAP_MAGIC {
        bail!("Invalid raw model magic in {}: {:?}", path.display(), &magic[..]);
    }

    // Skip nVectors (u32) - unused during read
    let _n_vectors = reader.read_u32::<LittleEndian>()?;

    // Read nGroups
    let n_groups = reader.read_u32::<LittleEndian>()?;

    // Read RootWMOID
    let root_wmo_id = reader.read_u32::<LittleEndian>()?;

    // Read groups
    let mut groups = Vec::with_capacity(n_groups as usize);
    for _ in 0..n_groups {
        match read_raw_group(&mut reader) {
            Ok(group) => groups.push(group),
            Err(e) => {
                warn!("Failed to read group in {}: {}", path.display(), e);
                break;
            }
        }
    }

    Ok(RawWorldModel { root_wmo_id, groups })
}

/// Read a raw group (equivalent to MaNGOS GroupModel_Raw::Read)
fn read_raw_group<R: Read>(reader: &mut R) -> Result<RawGroupModel> {
    // mogpFlags
    let mogp_flags = reader.read_u32::<LittleEndian>()?;

    // GroupWMOID
    let group_wmo_id = reader.read_u32::<LittleEndian>()?;

    // bounds (AABox = 6 floats: min xyz, max xyz)
    let min_x = reader.read_f32::<LittleEndian>()?;
    let min_y = reader.read_f32::<LittleEndian>()?;
    let min_z = reader.read_f32::<LittleEndian>()?;
    let max_x = reader.read_f32::<LittleEndian>()?;
    let max_y = reader.read_f32::<LittleEndian>()?;
    let max_z = reader.read_f32::<LittleEndian>()?;
    let bounds = BoundingBox::new(
        Vec3::new(min_x, min_y, min_z),
        Vec3::new(max_x, max_y, max_z),
    );

    // liquidflags
    let liquid_flags = reader.read_u32::<LittleEndian>()?;

    // "GRP " chunk
    let mut chunk_id = [0u8; 4];
    reader.read_exact(&mut chunk_id)?;
    if &chunk_id != b"GRP " {
        bail!("Expected GRP chunk, got {:?}", std::str::from_utf8(&chunk_id));
    }
    let _block_size = reader.read_u32::<LittleEndian>()?;
    let branches = reader.read_u32::<LittleEndian>()?;
    for _ in 0..branches {
        let _branch_val = reader.read_u32::<LittleEndian>()?;
    }

    // "INDX" chunk
    reader.read_exact(&mut chunk_id)?;
    if &chunk_id != b"INDX" {
        bail!("Expected INDX chunk, got {:?}", std::str::from_utf8(&chunk_id));
    }
    let _block_size = reader.read_u32::<LittleEndian>()?;
    let n_indexes = reader.read_u32::<LittleEndian>()?;
    let mut triangles = Vec::with_capacity((n_indexes / 3) as usize);
    if n_indexes > 0 {
        let mut index_array = vec![0u16; n_indexes as usize];
        for i in 0..n_indexes as usize {
            index_array[i] = reader.read_u16::<LittleEndian>()?;
        }
        for i in (0..n_indexes as usize).step_by(3) {
            triangles.push(MeshTriangle {
                idx0: index_array[i] as u32,
                idx1: index_array[i + 1] as u32,
                idx2: index_array[i + 2] as u32,
            });
        }
    }

    // "VERT" chunk
    reader.read_exact(&mut chunk_id)?;
    if &chunk_id != b"VERT" {
        bail!("Expected VERT chunk, got {:?}", std::str::from_utf8(&chunk_id));
    }
    let _block_size = reader.read_u32::<LittleEndian>()?;
    let n_vectors = reader.read_u32::<LittleEndian>()?;
    let mut vertices = Vec::with_capacity(n_vectors as usize);
    for _ in 0..n_vectors {
        let x = reader.read_f32::<LittleEndian>()?;
        let y = reader.read_f32::<LittleEndian>()?;
        let z = reader.read_f32::<LittleEndian>()?;
        vertices.push(Vec3::new(x, y, z));
    }

    // Liquid data (optional)
    let liquid = if liquid_flags & 1 != 0 {
        reader.read_exact(&mut chunk_id)?;
        if &chunk_id != b"LIQU" {
            bail!("Expected LIQU chunk, got {:?}", std::str::from_utf8(&chunk_id));
        }
        let _block_size = reader.read_u32::<LittleEndian>()?;

        // WMOLiquidHeader: xverts(i32), yverts(i32), xtiles(i32), ytiles(i32), pos(3f), type(u16)
        let xverts = reader.read_i32::<LittleEndian>()?;
        let yverts = reader.read_i32::<LittleEndian>()?;
        let xtiles = reader.read_i32::<LittleEndian>()?;
        let ytiles = reader.read_i32::<LittleEndian>()?;
        let pos_x = reader.read_f32::<LittleEndian>()?;
        let pos_y = reader.read_f32::<LittleEndian>()?;
        let pos_z = reader.read_f32::<LittleEndian>()?;
        let liq_type = reader.read_u16::<LittleEndian>()?;

        // Read height data: (xverts * yverts) floats
        let height_count = (xverts * yverts) as usize;
        let mut heights = vec![0.0f32; height_count];
        for h in &mut heights {
            *h = reader.read_f32::<LittleEndian>()?;
        }

        // Read flag data: (xtiles * ytiles) bytes
        let flag_count = (xtiles * ytiles) as usize;
        let mut flags = vec![0u8; flag_count];
        reader.read_exact(&mut flags)?;

        Some(WmoLiquid {
            tiles_x: xtiles as u32,
            tiles_y: ytiles as u32,
            corner: Vec3::new(pos_x, pos_y, pos_z),
            liquid_type: liq_type as u32,
            heights,
            flags,
        })
    } else {
        None
    };

    Ok(RawGroupModel {
        mogp_flags,
        group_wmo_id,
        bounds,
        vertices,
        triangles,
        liquid,
    })
}

/// Write a .vmo file in VMAP_7.0 server format
/// Equivalent to MaNGOS TileAssembler::convertRawFile() + WorldModel::writeFile()
fn write_vmo(model: &RawWorldModel, path: &Path) -> Result<()> {
    let file = File::create(path)
        .with_context(|| format!("Failed to create .vmo file: {}", path.display()))?;
    let mut writer = BufWriter::new(file);

    // Write magic
    writer.write_all(VMAP_MAGIC)?;

    // Write "WMOD" chunk
    writer.write_all(b"WMOD")?;
    let chunk_size: u32 = 8; // sizeof(uint32) + sizeof(uint32) in MaNGOS, but only RootWMOID is read
    writer.write_u32::<LittleEndian>(chunk_size)?;
    writer.write_u32::<LittleEndian>(model.root_wmo_id)?;

    if !model.groups.is_empty() {
        // Write "GMOD" chunk
        writer.write_all(b"GMOD")?;
        let count = model.groups.len() as u32;
        writer.write_u32::<LittleEndian>(count)?;

        // Write each group
        for group in &model.groups {
            write_group_model(&mut writer, group)?;
        }

        // Write "GBIH" chunk - BIH tree over all groups
        writer.write_all(b"GBIH")?;

        // Build BIH from group bounding boxes
        let group_bounds: Vec<BoundingBox> = model.groups.iter()
            .map(|g| g.bounds)
            .collect();

        if group_bounds.is_empty() {
            // Empty BIH
            let empty_bih = BIH::new();
            empty_bih.write_to_file(&mut writer)?;
        } else {
            let (bih, _stats) = BIH::build(group_bounds.len(), |i| group_bounds[i]);
            bih.write_to_file(&mut writer)?;
        }
    }

    writer.flush()?;
    Ok(())
}

/// Write a single group model in server format
/// Equivalent to MaNGOS GroupModel::writeToFile()
fn write_group_model<W: Write>(writer: &mut W, group: &RawGroupModel) -> Result<()> {
    // Write bounding box (AABox = 6 floats)
    writer.write_f32::<LittleEndian>(group.bounds.min.x)?;
    writer.write_f32::<LittleEndian>(group.bounds.min.y)?;
    writer.write_f32::<LittleEndian>(group.bounds.min.z)?;
    writer.write_f32::<LittleEndian>(group.bounds.max.x)?;
    writer.write_f32::<LittleEndian>(group.bounds.max.y)?;
    writer.write_f32::<LittleEndian>(group.bounds.max.z)?;

    // Write mogpFlags
    writer.write_u32::<LittleEndian>(group.mogp_flags)?;

    // Write GroupWMOID
    writer.write_u32::<LittleEndian>(group.group_wmo_id)?;

    // Write "VERT" chunk
    writer.write_all(b"VERT")?;
    let vert_count = group.vertices.len() as u32;
    let vert_chunk_size = 4 + vert_count * 12; // sizeof(u32) + count * sizeof(Vector3)
    writer.write_u32::<LittleEndian>(vert_chunk_size)?;
    writer.write_u32::<LittleEndian>(vert_count)?;

    if vert_count == 0 {
        // Models without geometry end here (matches MaNGOS early return)
        return Ok(());
    }

    for v in &group.vertices {
        writer.write_f32::<LittleEndian>(v.x)?;
        writer.write_f32::<LittleEndian>(v.y)?;
        writer.write_f32::<LittleEndian>(v.z)?;
    }

    // Write "TRIM" chunk (triangle mesh)
    writer.write_all(b"TRIM")?;
    let tri_count = group.triangles.len() as u32;
    let tri_chunk_size = 4 + tri_count * 12; // sizeof(u32) + count * sizeof(MeshTriangle)
    writer.write_u32::<LittleEndian>(tri_chunk_size)?;
    writer.write_u32::<LittleEndian>(tri_count)?;
    for tri in &group.triangles {
        writer.write_u32::<LittleEndian>(tri.idx0)?;
        writer.write_u32::<LittleEndian>(tri.idx1)?;
        writer.write_u32::<LittleEndian>(tri.idx2)?;
    }

    // Write "MBIH" chunk (per-group mesh BIH tree)
    writer.write_all(b"MBIH")?;

    if group.triangles.is_empty() || group.vertices.is_empty() {
        // Empty BIH for groups with no mesh
        let empty_bih = BIH::new();
        empty_bih.write_to_file(writer)?;
    } else {
        // Build BIH from triangle bounding boxes
        let tri_bounds: Vec<BoundingBox> = group.triangles.iter()
            .map(|tri| {
                let v0 = group.vertices[tri.idx0 as usize];
                let v1 = group.vertices[tri.idx1 as usize];
                let v2 = group.vertices[tri.idx2 as usize];
                BoundingBox::new(
                    Vec3::new(v0.x.min(v1.x).min(v2.x), v0.y.min(v1.y).min(v2.y), v0.z.min(v1.z).min(v2.z)),
                    Vec3::new(v0.x.max(v1.x).max(v2.x), v0.y.max(v1.y).max(v2.y), v0.z.max(v1.z).max(v2.z)),
                )
            })
            .collect();

        let (bih, _stats) = BIH::build(tri_bounds.len(), |i| tri_bounds[i]);
        bih.write_to_file(writer)?;
    }

    // Write "LIQU" chunk
    writer.write_all(b"LIQU")?;
    if let Some(ref liquid) = group.liquid {
        let chunk_size = liquid.file_size();
        writer.write_u32::<LittleEndian>(chunk_size)?;
        liquid.write_to(writer)?;
    } else {
        writer.write_u32::<LittleEndian>(0)?;
    }

    Ok(())
}

/// Convert a single raw model file to .vmo format
/// Returns Ok(true) if conversion succeeded, Ok(false) if file was skipped
pub fn convert_raw_file(raw_path: &Path, vmo_path: &Path) -> Result<bool> {
    if !raw_path.exists() {
        return Ok(false);
    }

    let raw_model = match read_raw_model(raw_path) {
        Ok(m) => m,
        Err(e) => {
            debug!("Skipping {}: {}", raw_path.display(), e);
            return Ok(false);
        }
    };

    write_vmo(&raw_model, vmo_path)?;
    Ok(true)
}

/// Convert all raw model files referenced by dir_bin entries
/// Returns the number of models converted
pub fn convert_all_models(
    entries: &[crate::vmaps::dir_bin::DirBinEntry],
    buildings_dir: &Path,
    vmaps_dir: &Path,
) -> Result<usize> {
    use std::collections::HashSet;

    // Collect unique model names
    let mut model_names: HashSet<String> = HashSet::new();
    for entry in entries {
        if !entry.name.is_empty() {
            model_names.insert(entry.name.clone());
        }
    }

    let total = model_names.len();
    let mut converted = 0;
    let mut skipped = 0;

    let pb = indicatif::ProgressBar::new(total as u64);
    pb.set_style(
        indicatif::ProgressStyle::with_template(
            "  Converting models [{bar:40}] {pos}/{len} ({eta})"
        )
        .unwrap()
        .progress_chars("=> "),
    );

    for name in &model_names {
        // Try exact name first, then with .wmo extension (handles old dir_bin without extension)
        let raw_path = {
            let exact = buildings_dir.join(name);
            if exact.exists() {
                exact
            } else {
                buildings_dir.join(format!("{}.wmo", name))
            }
        };
        let vmo_path = vmaps_dir.join(format!("{}.vmo", name));

        // Create parent directory if needed (model names can have subdirs)
        if let Some(parent) = vmo_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        match convert_raw_file(&raw_path, &vmo_path) {
            Ok(true) => converted += 1,
            Ok(false) => skipped += 1,
            Err(e) => {
                debug!("Failed to convert {}: {}", name, e);
                skipped += 1;
            }
        }

        pb.inc(1);
    }

    pb.finish_and_clear();

    if skipped > 0 {
        debug!("Skipped {} models during conversion", skipped);
    }

    Ok(converted)
}

/// Read raw model and return its vertices for bounding box calculation
/// Used by the assembly step to compute transformed bounds for M2 models
pub fn read_raw_model_vertices(path: &Path) -> Result<Vec<Vec3>> {
    let model = read_raw_model(path)?;
    let mut all_vertices = Vec::new();
    for group in &model.groups {
        all_vertices.extend_from_slice(&group.vertices);
    }
    Ok(all_vertices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_triangle_layout() {
        // MeshTriangle is 3 u32s = 12 bytes
        assert_eq!(std::mem::size_of::<MeshTriangle>(), 12);
    }

    #[test]
    fn test_wmo_liquid_file_size() {
        let liquid = WmoLiquid {
            tiles_x: 2,
            tiles_y: 3,
            corner: Vec3::ZERO,
            liquid_type: 0,
            heights: vec![0.0; 12], // (2+1)*(3+1) = 12
            flags: vec![0; 6],     // 2*3 = 6
        };

        // 4+4+12+4 + 12*4 + 6 = 24 + 48 + 6 = 78
        assert_eq!(liquid.file_size(), 78);
    }

    #[test]
    fn test_roundtrip_raw_to_vmo() {
        // Create a minimal raw VMAPs05 file in memory
        let mut raw_data = Vec::new();
        let mut cursor = Cursor::new(&mut raw_data);

        // Magic
        cursor.write_all(b"VMAPs05\0").unwrap();
        // nVectors (unused)
        cursor.write_u32::<LittleEndian>(3).unwrap();
        // nGroups
        cursor.write_u32::<LittleEndian>(1).unwrap();
        // RootWMOID
        cursor.write_u32::<LittleEndian>(42).unwrap();

        // Group:
        // mogpFlags
        cursor.write_u32::<LittleEndian>(0).unwrap();
        // GroupWMOID
        cursor.write_u32::<LittleEndian>(1).unwrap();
        // bounds (6 floats)
        for v in &[0.0f32, 0.0, 0.0, 1.0, 1.0, 1.0] {
            cursor.write_f32::<LittleEndian>(*v).unwrap();
        }
        // liquidflags
        cursor.write_u32::<LittleEndian>(0).unwrap();
        // "GRP " chunk
        cursor.write_all(b"GRP ").unwrap();
        cursor.write_u32::<LittleEndian>(8).unwrap(); // blocksize
        cursor.write_u32::<LittleEndian>(1).unwrap(); // branches
        cursor.write_u32::<LittleEndian>(3).unwrap(); // branch value
        // "INDX" chunk
        cursor.write_all(b"INDX").unwrap();
        cursor.write_u32::<LittleEndian>(4 + 3 * 2).unwrap(); // blocksize
        cursor.write_u32::<LittleEndian>(3).unwrap(); // nindexes
        cursor.write_u16::<LittleEndian>(0).unwrap();
        cursor.write_u16::<LittleEndian>(1).unwrap();
        cursor.write_u16::<LittleEndian>(2).unwrap();
        // "VERT" chunk
        cursor.write_all(b"VERT").unwrap();
        cursor.write_u32::<LittleEndian>(4 + 3 * 12).unwrap(); // blocksize
        cursor.write_u32::<LittleEndian>(3).unwrap(); // nvectors
        for v in &[0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            cursor.write_f32::<LittleEndian>(*v).unwrap();
        }

        // Write raw file to temp
        let raw_path = std::env::temp_dir().join("test_raw.wmo");
        std::fs::write(&raw_path, &raw_data).unwrap();

        // Read it back
        let model = read_raw_model(&raw_path).unwrap();
        assert_eq!(model.root_wmo_id, 42);
        assert_eq!(model.groups.len(), 1);
        assert_eq!(model.groups[0].vertices.len(), 3);
        assert_eq!(model.groups[0].triangles.len(), 1);

        // Convert to .vmo
        let vmo_path = std::env::temp_dir().join("test_output.vmo");
        write_vmo(&model, &vmo_path).unwrap();

        // Verify .vmo starts with VMAP_7.0 magic
        let vmo_data = std::fs::read(&vmo_path).unwrap();
        assert_eq!(&vmo_data[0..8], b"VMAP_7.0");
        // Verify WMOD chunk
        assert_eq!(&vmo_data[8..12], b"WMOD");
        // Verify GMOD chunk
        let gmod_pos = vmo_data.windows(4).position(|w| w == b"GMOD").unwrap();
        assert!(gmod_pos > 0);

        // Cleanup
        std::fs::remove_file(&raw_path).ok();
        std::fs::remove_file(&vmo_path).ok();
    }
}
