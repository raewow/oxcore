//! WMO File Parser
//!
//! Parses WMO root and group files from binary data.
//! Reference: mangos/contrib/vmap_extractor/vmapextract/vmapexport.cpp

use anyhow::{bail, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use glam::{Quat, Vec2, Vec3};
use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::vmaps::types::{
    BoundingBox, WMOBatch, WMODoodadDef, WMODoodadSet, WMOGroupInfo, WMOMaterial,
};
use crate::vmaps::wmo::group::{WMOGroup, MOGPHeader};
use crate::vmaps::wmo::root::{WMORoot, MOHDHeader};

// WMO Root chunk fourcc codes
const MVER: &[u8; 4] = b"MVER"; // Version
const MOHD: &[u8; 4] = b"MOHD"; // Header
const MOTX: &[u8; 4] = b"MOTX"; // Textures
const MOMT: &[u8; 4] = b"MOMT"; // Materials
const MOGN: &[u8; 4] = b"MOGN"; // Group names
const MOGI: &[u8; 4] = b"MOGI"; // Group info
const MOSB: &[u8; 4] = b"MOSB"; // Skybox
const MOPV: &[u8; 4] = b"MOPV"; // Portal vertices
const MOPT: &[u8; 4] = b"MOPT"; // Portal info
const MOPR: &[u8; 4] = b"MOPR"; // Portal references
const MOVV: &[u8; 4] = b"MOVV"; // Visible vertices
const MOVB: &[u8; 4] = b"MOVB"; // Visible blocks
const MOLT: &[u8; 4] = b"MOLT"; // Lights
const MODS: &[u8; 4] = b"MODS"; // Doodad sets
const MODN: &[u8; 4] = b"MODN"; // Doodad names
const MODD: &[u8; 4] = b"MODD"; // Doodad definitions
const MFOG: &[u8; 4] = b"MFOG"; // Fog
const MCVP: &[u8; 4] = b"MCVP"; // Convex volume planes

// WMO Group chunk fourcc codes
const MOGP: &[u8; 4] = b"MOGP"; // Group header
const MOPY: &[u8; 4] = b"MOPY"; // Material info for triangles
const MOVI: &[u8; 4] = b"MOVI"; // Vertex indices
const MOVT: &[u8; 4] = b"MOVT"; // Vertices
const MONR: &[u8; 4] = b"MONR"; // Normals
const MOTV: &[u8; 4] = b"MOTV"; // Texture coords
const MOBA: &[u8; 4] = b"MOBA"; // Render batches
const MOLR: &[u8; 4] = b"MOLR"; // Light references
const MODR: &[u8; 4] = b"MODR"; // Doodad references
const MOBN: &[u8; 4] = b"MOBN"; // BSP nodes
const MOBR: &[u8; 4] = b"MOBR"; // BSP face indices
const MOCV: &[u8; 4] = b"MOCV"; // Vertex colors
const MLIQ: &[u8; 4] = b"MLIQ"; // Liquids

impl WMORoot {
    /// Parse WMO root file from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        let mut root = WMORoot::new();

        // Read and verify version
        let version = read_mver(&mut cursor)?;
        if version != 17 {
            bail!("Unsupported WMO version: {} (expected 17)", version);
        }

        // Read MOHD header - this contains counts for other chunks
        let header = read_mohd(&mut cursor)?;
        root.n_groups = header.n_groups;
        root.n_portals = header.n_portals;
        root.n_lights = header.n_lights;
        root.n_models = header.n_models;
        root.n_doodads = header.n_doodads;
        root.n_sets = header.n_sets;
        root.ambient_color = header.ambient_color;
        root.wmo_id = header.wmo_id;
        root.bounding_box = header.bounding_box;
        root.flags = header.flags;

        // Read remaining chunks (order may vary)
        while cursor.position() < data.len() as u64 {
            let chunk_start = cursor.position();

            // Try to read chunk header
            let chunk_header = match read_chunk_header(&mut cursor) {
                Ok(header) => header,
                Err(_) => break, // End of file
            };

            let (fourcc, size) = chunk_header;
            let fourcc_str = String::from_utf8_lossy(&fourcc);

            match &fourcc {
                MOTX => {
                    // Textures - skip for now
                    cursor.seek(SeekFrom::Current(size as i64))
                        .with_context(|| format!("Failed to skip MOTX chunk (size: {})", size))?;
                }
                MOMT => {
                    root.materials = read_momt(&mut cursor, size, header.n_materials)
                        .with_context(|| format!("Failed to read MOMT chunk (size: {}, expected materials: {})", size, header.n_materials))?;
                }
                MOGN => {
                    root.group_names = read_mogn(&mut cursor, size)
                        .with_context(|| format!("Failed to read MOGN chunk (size: {})", size))?;
                }
                MOGI => {
                    root.group_info = read_mogi(&mut cursor, size)
                        .with_context(|| format!("Failed to read MOGI chunk (size: {})", size))?;
                }
                MODN => {
                    root.doodad_names = read_modn(&mut cursor, size)
                        .with_context(|| format!("Failed to read MODN chunk (size: {})", size))?;
                }
                MODD => {
                    root.doodad_defs = read_modd(&mut cursor, size)
                        .with_context(|| format!("Failed to read MODD chunk (size: {})", size))?;
                }
                MODS => {
                    root.doodad_sets = read_mods(&mut cursor, size)
                        .with_context(|| format!("Failed to read MODS chunk (size: {})", size))?;
                }
                _ => {
                    // Skip unknown chunks
                    cursor.seek(SeekFrom::Current(size as i64))
                        .with_context(|| format!("Failed to skip unknown chunk '{}' (size: {})", fourcc_str, size))?;
                }
            }

            // Ensure we're at the right position (some chunks may not consume all data)
            let expected_pos = chunk_start + 8 + size as u64;
            cursor.set_position(expected_pos);
        }

        Ok(root)
    }
}

impl WMOGroup {
    /// Parse WMO group file from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        let mut group = WMOGroup::new();

        // Read and verify version
        let version = read_mver(&mut cursor)?;
        if version != 17 {
            bail!("Unsupported WMO group version: {} (expected 17)", version);
        }

        // Read MOGP chunk (this is a container for all subchunks)
        let (fourcc, mogp_size) = read_chunk_header(&mut cursor)?;
        if &fourcc != MOGP {
            bail!("Expected MOGP chunk, got '{}'", String::from_utf8_lossy(&fourcc));
        }

        let mogp_start = cursor.position();
        let mogp_end = mogp_start + mogp_size as u64;

        // Read MOGP header
        let header = read_mogp_header(&mut cursor)?;
        group.mogp_flags = header.flags;
        group.group_wmo_id = header.group_id;
        group.bounding_box = header.bounding_box;
        group.name_offset = header.group_name_offset;
        group.desc_group_name = header.desc_group_name;
        group.liquid_type = header.liquid_type;

        // Read subchunks within MOGP
        while cursor.position() < mogp_end {
            let chunk_start = cursor.position();

            let chunk_header = match read_chunk_header(&mut cursor) {
                Ok(header) => header,
                Err(_) => break,
            };

            let (fourcc, size) = chunk_header;

            match &fourcc {
                MOPY => {
                    group.materials = read_mopy(&mut cursor, size)?;
                }
                MOVI => {
                    group.indices = read_movi(&mut cursor, size)?;
                }
                MOVT => {
                    group.vertices = read_movt(&mut cursor, size)?;
                }
                MONR => {
                    group.normals = read_monr(&mut cursor, size)?;
                }
                MOTV => {
                    group.tex_coords = read_motv(&mut cursor, size)?;
                }
                MOBA => {
                    group.batch_info = read_moba(&mut cursor, size)?;
                }
                MODR => {
                    group.doodad_references = read_modr(&mut cursor, size)?;
                }
                MLIQ => {
                    // Liquid data - can be parsed later if needed
                    cursor.seek(SeekFrom::Current(size as i64))?;
                }
                _ => {
                    // Skip unknown chunks
                    cursor.seek(SeekFrom::Current(size as i64))?;
                }
            }

            // Ensure we're at the right position
            let expected_pos = chunk_start + 8 + size as u64;
            cursor.set_position(expected_pos);
        }

        Ok(group)
    }
}

/// Read chunk header (fourcc + size)
fn read_chunk_header(cursor: &mut Cursor<&[u8]>) -> Result<([u8; 4], u32)> {
    let mut fourcc = [0u8; 4];
    cursor.read_exact(&mut fourcc)
        .context("Failed to read chunk fourcc")?;

    // Reverse fourcc bytes (WMO files store them reversed)
    fourcc.reverse();

    let size = cursor.read_u32::<LittleEndian>()
        .context("Failed to read chunk size")?;

    Ok((fourcc, size))
}

/// Read MVER chunk (version)
fn read_mver(cursor: &mut Cursor<&[u8]>) -> Result<u32> {
    let (fourcc, size) = read_chunk_header(cursor)?;

    if &fourcc != MVER {
        bail!("Expected MVER chunk, got '{}'", String::from_utf8_lossy(&fourcc));
    }

    if size != 4 {
        bail!("Invalid MVER size: {} (expected 4)", size);
    }

    cursor.read_u32::<LittleEndian>()
        .context("Failed to read version")
}

/// Read MOHD chunk (root header)
fn read_mohd(cursor: &mut Cursor<&[u8]>) -> Result<MOHDHeader> {
    let (fourcc, size) = read_chunk_header(cursor)?;

    if &fourcc != MOHD {
        bail!("Expected MOHD chunk, got '{}'", String::from_utf8_lossy(&fourcc));
    }

    if size < 64 {
        bail!("MOHD chunk too small: {} (expected at least 64)", size);
    }

    let n_materials = cursor.read_u32::<LittleEndian>()
        .context("Failed to read n_materials")?;
    let n_groups = cursor.read_u32::<LittleEndian>()
        .context("Failed to read n_groups")?;
    let n_portals = cursor.read_u32::<LittleEndian>()
        .context("Failed to read n_portals")?;
    let n_lights = cursor.read_u32::<LittleEndian>()
        .context("Failed to read n_lights")?;
    let n_models = cursor.read_u32::<LittleEndian>()
        .context("Failed to read n_models")?;
    let n_doodads = cursor.read_u32::<LittleEndian>()
        .context("Failed to read n_doodads")?;
    let n_sets = cursor.read_u32::<LittleEndian>()
        .context("Failed to read n_sets")?;
    let ambient_color = cursor.read_u32::<LittleEndian>()
        .context("Failed to read ambient_color")?;
    let wmo_id = cursor.read_u32::<LittleEndian>()
        .context("Failed to read wmo_id")?;

    // Read bounding box
    let bbox_min = Vec3::new(
        cursor.read_f32::<LittleEndian>().context("Failed to read bbox_min.x")?,
        cursor.read_f32::<LittleEndian>().context("Failed to read bbox_min.y")?,
        cursor.read_f32::<LittleEndian>().context("Failed to read bbox_min.z")?,
    );
    let bbox_max = Vec3::new(
        cursor.read_f32::<LittleEndian>().context("Failed to read bbox_max.x")?,
        cursor.read_f32::<LittleEndian>().context("Failed to read bbox_max.y")?,
        cursor.read_f32::<LittleEndian>().context("Failed to read bbox_max.z")?,
    );

    let flags = cursor.read_u32::<LittleEndian>()
        .context("Failed to read flags")?;

    // Skip any remaining bytes in MOHD chunk (some files have larger MOHD)
    // We've read 64 bytes (9 u32s + 6 f32s + 1 u32), skip the rest
    if size > 64 {
        cursor.seek(SeekFrom::Current((size - 64) as i64))
            .context("Failed to skip remaining MOHD data")?;
    }

    Ok(MOHDHeader {
        n_materials,
        n_groups,
        n_portals,
        n_lights,
        n_models,
        n_doodads,
        n_sets,
        ambient_color,
        wmo_id,
        bounding_box: BoundingBox::new(bbox_min, bbox_max),
        flags,
    })
}

/// Read MOMT chunk (materials)
fn read_momt(cursor: &mut Cursor<&[u8]>, size: u32, n_materials: u32) -> Result<Vec<WMOMaterial>> {
    let material_size = 64; // Size of material structure
    let expected_size = n_materials * material_size;

    if size < expected_size {
        bail!("MOMT size mismatch: {} < {}", size, expected_size);
    }

    let mut materials = Vec::with_capacity(n_materials as usize);

    for _ in 0..n_materials {
        materials.push(WMOMaterial {
            flags: cursor.read_u32::<LittleEndian>()?,
            shader: cursor.read_u32::<LittleEndian>()?,
            blend_mode: cursor.read_u32::<LittleEndian>()?,
            texture1: cursor.read_u32::<LittleEndian>()?,
            color1: cursor.read_u32::<LittleEndian>()?,
            texture2: cursor.read_u32::<LittleEndian>()?,
            color2: cursor.read_u32::<LittleEndian>()?,
            ground_type: cursor.read_u32::<LittleEndian>()?,
        });

        // Skip remaining material data (64 bytes total)
        cursor.seek(SeekFrom::Current(32))?;
    }

    Ok(materials)
}

/// Read MOGN chunk (group names)
fn read_mogn(cursor: &mut Cursor<&[u8]>, size: u32) -> Result<Vec<String>> {
    let mut buffer = vec![0u8; size as usize];
    cursor.read_exact(&mut buffer)?;

    // Parse null-terminated strings
    let mut names = Vec::new();
    let mut current = Vec::new();

    for &byte in &buffer {
        if byte == 0 {
            if !current.is_empty() {
                if let Ok(name) = String::from_utf8(current.clone()) {
                    names.push(name);
                }
                current.clear();
            }
        } else {
            current.push(byte);
        }
    }

    Ok(names)
}

/// Read MOGI chunk (group info)
fn read_mogi(cursor: &mut Cursor<&[u8]>, size: u32) -> Result<Vec<WMOGroupInfo>> {
    let entry_size = 32; // Size of group info structure
    let n_groups = size / entry_size;

    let mut group_info = Vec::with_capacity(n_groups as usize);

    for _ in 0..n_groups {
        let flags = cursor.read_u32::<LittleEndian>()?;

        let bbox_min = Vec3::new(
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
        );
        let bbox_max = Vec3::new(
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
        );

        let name_offset = cursor.read_u32::<LittleEndian>()?;

        group_info.push(WMOGroupInfo {
            flags,
            bounding_box: BoundingBox::new(bbox_min, bbox_max),
            name_offset,
        });
    }

    Ok(group_info)
}

/// Read MODN chunk (doodad names)
fn read_modn(cursor: &mut Cursor<&[u8]>, size: u32) -> Result<Vec<String>> {
    read_mogn(cursor, size) // Same format as MOGN
}

/// Read MODD chunk (doodad definitions)
fn read_modd(cursor: &mut Cursor<&[u8]>, size: u32) -> Result<Vec<WMODoodadDef>> {
    // Doodad definition structure (C++ reference):
    // struct MODD {
    //     uint32 NameIndex : 24;  // 24-bit bitfield in a 4-byte field
    //     Vec3D Position;         // 12 bytes (3 floats)
    //     Quaternion Rotation;    // 16 bytes (4 floats)
    //     float Scale;            // 4 bytes
    //     uint32 Color;           // 4 bytes
    // };
    // Total: 40 bytes
    const ENTRY_SIZE: u32 = 40;

    if size % ENTRY_SIZE != 0 {
        bail!("MODD chunk size {} is not a multiple of entry size {}", size, ENTRY_SIZE);
    }

    let n_doodads = size / ENTRY_SIZE;
    let mut doodads = Vec::with_capacity(n_doodads as usize);

    for _ in 0..n_doodads {
        // Read the 32-bit value that contains the 24-bit name index
        // (upper 8 bits are flags/padding, lower 24 bits are name index)
        let name_and_flags = cursor.read_u32::<LittleEndian>()?;
        let name_index = name_and_flags & 0x00FFFFFF; // Extract lower 24 bits
        let flags = (name_and_flags >> 24) & 0xFF;    // Extract upper 8 bits

        let position = Vec3::new(
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
        );

        let rotation = Quat::from_xyzw(
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
        );

        let scale = cursor.read_f32::<LittleEndian>()?;
        let color = cursor.read_u32::<LittleEndian>()?;

        doodads.push(WMODoodadDef {
            name_index,
            flags,
            position,
            rotation,
            scale,
            color,
        });
    }

    Ok(doodads)
}

/// Read MODS chunk (doodad sets)
fn read_mods(cursor: &mut Cursor<&[u8]>, size: u32) -> Result<Vec<WMODoodadSet>> {
    let entry_size = 32; // Size of doodad set
    let n_sets = size / entry_size;

    let mut sets = Vec::with_capacity(n_sets as usize);

    for _ in 0..n_sets {
        let mut name = [0u8; 20];
        cursor.read_exact(&mut name)?;

        let start_index = cursor.read_u32::<LittleEndian>()?;
        let count = cursor.read_u32::<LittleEndian>()?;
        let unused = cursor.read_u32::<LittleEndian>()?;

        sets.push(WMODoodadSet {
            name,
            start_index,
            count,
            unused,
        });
    }

    Ok(sets)
}

/// Read MOGP header (group header)
fn read_mogp_header(cursor: &mut Cursor<&[u8]>) -> Result<MOGPHeader> {
    let group_name_offset = cursor.read_u32::<LittleEndian>()?;
    let desc_group_name = cursor.read_u32::<LittleEndian>()?;
    let flags = cursor.read_u32::<LittleEndian>()?;

    let bbox_min = Vec3::new(
        cursor.read_f32::<LittleEndian>()?,
        cursor.read_f32::<LittleEndian>()?,
        cursor.read_f32::<LittleEndian>()?,
    );
    let bbox_max = Vec3::new(
        cursor.read_f32::<LittleEndian>()?,
        cursor.read_f32::<LittleEndian>()?,
        cursor.read_f32::<LittleEndian>()?,
    );

    let portal_start = cursor.read_u16::<LittleEndian>()?;
    let portal_count = cursor.read_u16::<LittleEndian>()?;
    let trans_batch_count = cursor.read_u16::<LittleEndian>()?;
    let int_batch_count = cursor.read_u16::<LittleEndian>()?;
    let ext_batch_count = cursor.read_u16::<LittleEndian>()?;
    let padding = cursor.read_u16::<LittleEndian>()?;

    let mut fogs = [0u8; 4];
    cursor.read_exact(&mut fogs)?;

    let liquid_type = cursor.read_u32::<LittleEndian>()?;
    let group_id = cursor.read_u32::<LittleEndian>()?;
    let unknown1 = cursor.read_u32::<LittleEndian>()?;
    let unknown2 = cursor.read_u32::<LittleEndian>()?;

    Ok(MOGPHeader {
        group_name_offset,
        desc_group_name,
        flags,
        bounding_box: BoundingBox::new(bbox_min, bbox_max),
        portal_start,
        portal_count,
        trans_batch_count,
        int_batch_count,
        ext_batch_count,
        padding,
        fogs,
        liquid_type,
        group_id,
        unknown1,
        unknown2,
    })
}

/// Read MOPY chunk (triangle material info)
fn read_mopy(cursor: &mut Cursor<&[u8]>, size: u32) -> Result<Vec<u16>> {
    let entry_size = 2; // Each entry is a u16 material ID
    let n_triangles = size / entry_size;

    let mut materials = Vec::with_capacity(n_triangles as usize);

    for _ in 0..n_triangles {
        // First byte is material ID, second byte is flags
        let material_and_flags = cursor.read_u16::<LittleEndian>()?;
        let material_id = (material_and_flags & 0xFF) as u16;
        materials.push(material_id);
    }

    Ok(materials)
}

/// Read MOVI chunk (vertex indices)
fn read_movi(cursor: &mut Cursor<&[u8]>, size: u32) -> Result<Vec<u16>> {
    let n_indices = size / 2; // u16 indices

    let mut indices = Vec::with_capacity(n_indices as usize);

    for _ in 0..n_indices {
        indices.push(cursor.read_u16::<LittleEndian>()?);
    }

    Ok(indices)
}

/// Read MOVT chunk (vertices)
fn read_movt(cursor: &mut Cursor<&[u8]>, size: u32) -> Result<Vec<Vec3>> {
    let n_vertices = size / 12; // 3 floats per vertex

    let mut vertices = Vec::with_capacity(n_vertices as usize);

    for _ in 0..n_vertices {
        vertices.push(Vec3::new(
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
        ));
    }

    Ok(vertices)
}

/// Read MONR chunk (normals)
fn read_monr(cursor: &mut Cursor<&[u8]>, size: u32) -> Result<Vec<Vec3>> {
    read_movt(cursor, size) // Same format as MOVT
}

/// Read MOTV chunk (texture coordinates)
fn read_motv(cursor: &mut Cursor<&[u8]>, size: u32) -> Result<Vec<Vec2>> {
    let n_coords = size / 8; // 2 floats per coordinate

    let mut tex_coords = Vec::with_capacity(n_coords as usize);

    for _ in 0..n_coords {
        tex_coords.push(Vec2::new(
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
        ));
    }

    Ok(tex_coords)
}

/// Read MOBA chunk (render batches)
fn read_moba(cursor: &mut Cursor<&[u8]>, size: u32) -> Result<Vec<WMOBatch>> {
    // Each MOBA entry is variable sized, but contains at minimum:
    // - Bounding box (24 bytes: 6 floats)
    // - Start index (4 bytes)
    // - Count (2 bytes)
    // - Min index (2 bytes)
    // - Max index (2 bytes)
    // - Material ID (1 byte)
    // - Padding/flags (1 byte)
    // Total minimum: 36 bytes
    const MIN_ENTRY_SIZE: u32 = 36;

    if size < MIN_ENTRY_SIZE {
        return Ok(Vec::new()); // Empty MOBA chunk
    }

    let n_batches = size / MIN_ENTRY_SIZE;
    let mut batches = Vec::with_capacity(n_batches as usize);

    for _ in 0..n_batches {
        // Skip bounding box (24 bytes)
        cursor.seek(SeekFrom::Current(24))?;

        let start_index = cursor.read_u32::<LittleEndian>()?;
        let count = cursor.read_u16::<LittleEndian>()?;
        let min_index = cursor.read_u16::<LittleEndian>()?;
        let max_index = cursor.read_u16::<LittleEndian>()?;
        let material_id = cursor.read_u8()?;

        // Skip padding
        cursor.seek(SeekFrom::Current(1))?;

        batches.push(WMOBatch {
            start_index,
            count,
            min_index,
            max_index,
            material_id,
        });

        // Skip any remaining bytes in this entry (if entry size > MIN_ENTRY_SIZE)
        // This handles variable-sized entries
        let bytes_read = MIN_ENTRY_SIZE;
        if size / n_batches > bytes_read {
            let remaining = (size / n_batches) - bytes_read;
            cursor.seek(SeekFrom::Current(remaining as i64))?;
        }
    }

    Ok(batches)
}

/// Read MODR chunk (doodad references)
fn read_modr(cursor: &mut Cursor<&[u8]>, size: u32) -> Result<Vec<u16>> {
    let n_refs = size / 2; // u16 references

    let mut refs = Vec::with_capacity(n_refs as usize);

    for _ in 0..n_refs {
        refs.push(cursor.read_u16::<LittleEndian>()?);
    }

    Ok(refs)
}
