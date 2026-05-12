//! ADT File Structures
//!
//! ADT (Area Data Tile) files contain terrain data for World of Warcraft maps.
//! Each ADT file represents a 533.33 x 533.33 yard tile of the world.

use anyhow::{bail, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};
use std::path::Path;

// ADT grid constants
pub const ADT_CELLS_PER_GRID: usize = 16;
pub const ADT_CELL_SIZE: usize = 8;
pub const ADT_GRID_SIZE: usize = ADT_CELLS_PER_GRID * ADT_CELL_SIZE; // 128

// World space constants
pub const TILESIZE: f32 = 533.33333;
pub const CHUNKSIZE: f32 = TILESIZE / 16.0; // ~33.33
pub const UNITSIZE: f32 = CHUNKSIZE / 8.0;  // ~4.17

// Height map sizes
pub const V9_SIZE: usize = ADT_GRID_SIZE + 1; // 129 (vertices)
pub const V8_SIZE: usize = ADT_GRID_SIZE;     // 128 (cell centers)

/// ADT main header (MHDR chunk)
#[derive(Debug, Clone)]
pub struct AdtMHDR {
    pub pad: u32,
    pub offs_mcin: u32,
    pub offs_mtex: u32,
    pub offs_mmdx: u32,
    pub offs_mmid: u32,
    pub offs_mwmo: u32,
    pub offs_mwid: u32,
    pub offs_mddf: u32,
    pub offs_modf: u32,
    pub offs_mfbo: u32,
    pub offs_mh2o: u32,
    pub offs_mtxf: u32,
}

/// Cell index entry in MCIN chunk
#[derive(Debug, Clone, Copy)]
pub struct MCINCell {
    pub offs_mcnk: u32,
    pub size: u32,
    pub flags: u32,
    pub async_id: u32,
}

/// Cell index chunk (MCIN)
#[derive(Debug, Clone)]
pub struct AdtMCIN {
    pub cells: [[MCINCell; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID],
}

/// Cell terrain chunk (MCNK)
#[derive(Debug, Clone)]
pub struct AdtMCNK {
    pub flags: u32,
    pub ix: u32,
    pub iy: u32,
    pub n_layers: u32,
    pub n_doodad_refs: u32,
    pub offs_mcvt: u32,  // Height map offset (relative to MCNK start + 8)
    pub offs_mcnr: u32,  // Normals offset
    pub offs_mcly: u32,  // Texture layers offset
    pub offs_mcrf: u32,  // Doodad refs offset
    pub offs_mcal: u32,  // Alpha maps offset
    pub size_mcal: u32,
    pub offs_mcsh: u32,  // Shadow map offset
    pub size_mcsh: u32,
    pub area_id: u32,
    pub n_map_obj_refs: u32,
    pub holes: u16,      // Terrain holes bitmask
    pub low_quality_texture_map: u16,
    pub pred_tex: u32,
    pub no_effect_doodad: u32,
    pub offs_mcse: u32,
    pub n_sound_emitters: u32,
    pub offs_mclq: u32,  // Liquid offset
    pub size_mclq: u32,
    pub position: [f32; 3],
    pub offs_mccv: u32,  // Vertex colors
    pub offs_mclv: u32,  // Vertex lighting
    pub unused: u32,
}

/// Height map data (MCVT chunk)
/// Contains 145 height values: 9*9 (outer vertices) + 8*8 (inner vertices)
#[derive(Debug, Clone)]
pub struct AdtMCVT {
    pub heights: [f32; (ADT_CELL_SIZE + 1) * (ADT_CELL_SIZE + 1) + ADT_CELL_SIZE * ADT_CELL_SIZE],
}

impl AdtMCVT {
    /// Number of height values (145 for a standard chunk)
    pub const NUM_HEIGHTS: usize = (ADT_CELL_SIZE + 1) * (ADT_CELL_SIZE + 1) + ADT_CELL_SIZE * ADT_CELL_SIZE;
}

/// Liquid data entry (old MCLQ format)
#[derive(Debug, Clone, Copy)]
pub struct LiquidData {
    pub light: u32,
    pub height: f32,
}

/// Old liquid chunk (MCLQ)
#[derive(Debug, Clone)]
pub struct AdtMCLQ {
    pub height1: f32,
    pub height2: f32,
    pub liquid: [[LiquidData; ADT_CELL_SIZE + 1]; ADT_CELL_SIZE + 1],
    pub flags: [[u8; ADT_CELL_SIZE]; ADT_CELL_SIZE],
    pub data: [u8; 84],
}

/// New liquid chunk header (MH2O format - WotLK+)
#[derive(Debug, Clone)]
pub struct AdtMH2O {
    pub liquid: [[AdtLiquidInstance; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID],
}

/// Liquid instance reference
#[derive(Debug, Clone, Copy)]
pub struct AdtLiquidInstance {
    pub offs_data: u32,
    pub used: u32,
    pub offs_attributes: u32,
}

/// Liquid header data
#[derive(Debug, Clone)]
pub struct AdtLiquidHeader {
    pub liquid_type: u16,
    pub format_flags: u16,
    pub height_level1: f32,
    pub height_level2: f32,
    pub x_offset: u8,
    pub y_offset: u8,
    pub width: u8,
    pub height: u8,
    pub offs_data2a: u32,
    pub offs_data2b: u32,
}

/// M2 Model Placement (MDDF chunk)
#[derive(Debug, Clone)]
pub struct AdtMDDF {
    pub placements: Vec<M2Placement>,
}

/// M2 Model Placement Entry
#[derive(Debug, Clone)]
pub struct M2Placement {
    pub name_id: u32,      // Index into MMDX strings
    pub unique_id: u32,    // Unique instance ID
    pub position: [f32; 3],
    pub rotation: [f32; 3], // Euler angles in radians
    pub scale: u16,        // 1024 = 1.0x scale
    pub flags: u16,
}

/// WMO Placement (MODF chunk)
#[derive(Debug, Clone)]
pub struct AdtMODF {
    pub placements: Vec<WMOPlacement>,
}

/// WMO Placement Entry
#[derive(Debug, Clone)]
pub struct WMOPlacement {
    pub name_id: u32,      // Index into MWMO strings
    pub unique_id: u32,    // Unique instance ID
    pub position: [f32; 3],
    pub rotation: [f32; 3], // Euler angles in radians
    pub bounding_box_min: [f32; 3],
    pub bounding_box_max: [f32; 3],
    pub flags: u16,
    pub doodad_set: u16,   // Which doodad set to use
    pub name_set: u16,
    pub scale: u16,        // 1024 = 1.0x scale (added in later versions)
}

/// Complete ADT file
pub struct ADTFile {
    pub header: AdtMHDR,
    pub mcin: Option<AdtMCIN>,
    pub chunks: Vec<ChunkData>,
    pub mh2o: Option<AdtMH2O>,

    // Model/WMO filenames
    pub model_names: Vec<String>,  // MMDX
    pub wmo_names: Vec<String>,    // MWMO

    // Model/WMO placements
    pub mddf: Option<AdtMDDF>,     // M2 placements
    pub modf: Option<AdtMODF>,     // WMO placements

    raw_data: Vec<u8>,
}

/// Chunk data (combined MCNK + subchunks)
#[derive(Debug, Clone)]
pub struct ChunkData {
    pub mcnk: AdtMCNK,
    pub mcvt: Option<AdtMCVT>,
    pub mclq: Option<AdtMCLQ>,
}

impl ADTFile {
    /// Load an ADT file from disk
    pub fn load_file(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read ADT file: {}", path.display()))?;

        Self::from_bytes(data)
    }

    /// Parse an ADT file from bytes
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        let header = {
            let mut cursor = Cursor::new(data.as_slice());

            // Skip MVER chunk if present (8 bytes: fourcc + size, then 4 bytes version)
            let first_fourcc = read_fourcc(&mut cursor)?;
            let fourcc_str = String::from_utf8_lossy(&first_fourcc);
            if &first_fourcc == b"MVER" || fourcc_str == "REVM" {
                let _size = cursor.read_u32::<LittleEndian>()?;
                let _version = cursor.read_u32::<LittleEndian>()?;
            } else {
                // Reset to beginning if not MVER
                cursor.set_position(0);
            }

            read_mhdr(&mut cursor)?
        };

        // Parse MCIN chunk if it exists
        let mcin = if header.offs_mcin > 0 {
            let offset = 20 + header.offs_mcin as usize; // 20 = MVER(8) + MHDR header(8) + pad(4)
            Some(read_mcin(&data, offset)?)
        } else {
            None
        };

        // Parse all MCNK chunks
        let mut chunks = Vec::new();
        if let Some(ref mcin_data) = mcin {
            for y in 0..ADT_CELLS_PER_GRID {
                for x in 0..ADT_CELLS_PER_GRID {
                    let cell = &mcin_data.cells[y][x];
                    if cell.offs_mcnk > 0 {
                        // offset from file start
                        let offset = cell.offs_mcnk as usize;
                        if let Ok(chunk) = read_mcnk(&data, offset) {
                            chunks.push(chunk);
                        }
                    }
                }
            }
        }

        // Parse MH2O chunk if it exists (WotLK+ liquid format)
        let mh2o = if header.offs_mh2o > 0 {
            let offset = 20 + header.offs_mh2o as usize;
            read_mh2o(&data, offset).ok()
        } else {
            None
        };

        // Parse MMDX (M2 model names)
        let model_names = if header.offs_mmdx > 0 {
            let offset = 20 + header.offs_mmdx as usize;
            read_string_block(&data, offset).unwrap_or_default()
        } else {
            Vec::new()
        };

        // Parse MWMO (WMO names)
        let wmo_names = if header.offs_mwmo > 0 {
            let offset = 20 + header.offs_mwmo as usize;
            read_string_block(&data, offset).unwrap_or_default()
        } else {
            Vec::new()
        };

        // Parse MDDF (M2 placements)
        let mddf = if header.offs_mddf > 0 {
            let offset = 20 + header.offs_mddf as usize;
            read_mddf(&data, offset).ok()
        } else {
            None
        };

        // Parse MODF (WMO placements)
        let modf = if header.offs_modf > 0 {
            let offset = 20 + header.offs_modf as usize;
            read_modf(&data, offset).ok()
        } else {
            None
        };

        Ok(Self {
            header,
            mcin,
            chunks,
            mh2o,
            model_names,
            wmo_names,
            mddf,
            modf,
            raw_data: data,
        })
    }

    /// Get raw data at offset
    pub fn get_data_at(&self, offset: usize) -> &[u8] {
        if offset >= self.raw_data.len() {
            return &[];
        }
        &self.raw_data[offset..]
    }

    /// Get the number of chunks
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
}

/// Read a fourcc (4-character code)
fn read_fourcc(cursor: &mut Cursor<&[u8]>) -> Result<[u8; 4]> {
    let mut fourcc = [0u8; 4];
    cursor.read_exact(&mut fourcc)
        .context("Failed to read fourcc")?;
    Ok(fourcc)
}

/// Read MHDR chunk
fn read_mhdr(cursor: &mut Cursor<&[u8]>) -> Result<AdtMHDR> {
    // Read chunk header
    let fourcc = read_fourcc(cursor)?;
    let fourcc_str = String::from_utf8_lossy(&fourcc);
    if &fourcc != b"MHDR" && fourcc_str != "RDHM" {
        bail!("Expected MHDR chunk, got '{}'", fourcc_str);
    }

    let size = cursor.read_u32::<LittleEndian>()
        .context("Failed to read MHDR size")?;

    if size < 64 {
        bail!("MHDR chunk too small: {} bytes", size);
    }

    Ok(AdtMHDR {
        pad: cursor.read_u32::<LittleEndian>()?,
        offs_mcin: cursor.read_u32::<LittleEndian>()?,
        offs_mtex: cursor.read_u32::<LittleEndian>()?,
        offs_mmdx: cursor.read_u32::<LittleEndian>()?,
        offs_mmid: cursor.read_u32::<LittleEndian>()?,
        offs_mwmo: cursor.read_u32::<LittleEndian>()?,
        offs_mwid: cursor.read_u32::<LittleEndian>()?,
        offs_mddf: cursor.read_u32::<LittleEndian>()?,
        offs_modf: cursor.read_u32::<LittleEndian>()?,
        offs_mfbo: cursor.read_u32::<LittleEndian>()?,
        offs_mh2o: cursor.read_u32::<LittleEndian>()?,
        offs_mtxf: cursor.read_u32::<LittleEndian>()?,
    })
}

/// Read MCIN chunk (chunk index)
fn read_mcin(data: &[u8], offset: usize) -> Result<AdtMCIN> {
    if offset + 8 > data.len() {
        bail!("MCIN offset out of bounds");
    }

    let mut cursor = Cursor::new(&data[offset..]);
    let fourcc = read_fourcc(&mut cursor)?;
    let fourcc_str = String::from_utf8_lossy(&fourcc);
    if &fourcc != b"MCIN" && fourcc_str != "NICM" {
        bail!("Expected MCIN chunk, got '{}'", fourcc_str);
    }

    let _size = cursor.read_u32::<LittleEndian>()?;

    let mut cells = [[MCINCell {
        offs_mcnk: 0,
        size: 0,
        flags: 0,
        async_id: 0,
    }; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID];

    for y in 0..ADT_CELLS_PER_GRID {
        for x in 0..ADT_CELLS_PER_GRID {
            cells[y][x] = MCINCell {
                offs_mcnk: cursor.read_u32::<LittleEndian>()?,
                size: cursor.read_u32::<LittleEndian>()?,
                flags: cursor.read_u32::<LittleEndian>()?,
                async_id: cursor.read_u32::<LittleEndian>()?,
            };
        }
    }

    Ok(AdtMCIN { cells })
}

/// Read MCNK chunk (cell chunk)
fn read_mcnk(data: &[u8], offset: usize) -> Result<ChunkData> {
    if offset + 128 > data.len() {
        bail!("MCNK offset out of bounds");
    }

    let mut cursor = Cursor::new(&data[offset..]);
    let fourcc = read_fourcc(&mut cursor)?;
    if &fourcc != b"MCNK" {
        bail!("Expected MCNK chunk");
    }

    let _size = cursor.read_u32::<LittleEndian>()?;

    let mcnk = AdtMCNK {
        flags: cursor.read_u32::<LittleEndian>()?,
        ix: cursor.read_u32::<LittleEndian>()?,
        iy: cursor.read_u32::<LittleEndian>()?,
        n_layers: cursor.read_u32::<LittleEndian>()?,
        n_doodad_refs: cursor.read_u32::<LittleEndian>()?,
        offs_mcvt: cursor.read_u32::<LittleEndian>()?,
        offs_mcnr: cursor.read_u32::<LittleEndian>()?,
        offs_mcly: cursor.read_u32::<LittleEndian>()?,
        offs_mcrf: cursor.read_u32::<LittleEndian>()?,
        offs_mcal: cursor.read_u32::<LittleEndian>()?,
        size_mcal: cursor.read_u32::<LittleEndian>()?,
        offs_mcsh: cursor.read_u32::<LittleEndian>()?,
        size_mcsh: cursor.read_u32::<LittleEndian>()?,
        area_id: cursor.read_u32::<LittleEndian>()?,
        n_map_obj_refs: cursor.read_u32::<LittleEndian>()?,
        holes: cursor.read_u16::<LittleEndian>()?,
        low_quality_texture_map: cursor.read_u16::<LittleEndian>()?,
        pred_tex: cursor.read_u32::<LittleEndian>()?,
        no_effect_doodad: cursor.read_u32::<LittleEndian>()?,
        offs_mcse: cursor.read_u32::<LittleEndian>()?,
        n_sound_emitters: cursor.read_u32::<LittleEndian>()?,
        offs_mclq: cursor.read_u32::<LittleEndian>()?,
        size_mclq: cursor.read_u32::<LittleEndian>()?,
        position: [
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
        ],
        offs_mccv: cursor.read_u32::<LittleEndian>()?,
        offs_mclv: cursor.read_u32::<LittleEndian>()?,
        unused: cursor.read_u32::<LittleEndian>()?,
    };

    // Read MCVT subchunk if present
    let mcvt = if mcnk.offs_mcvt > 0 {
        let mcvt_offset = offset + mcnk.offs_mcvt as usize;
        read_mcvt(data, mcvt_offset).ok()
    } else {
        None
    };

    // Read MCLQ subchunk if present
    let mclq = if mcnk.offs_mclq > 0 && mcnk.size_mclq > 8 {
        let mclq_offset = offset + mcnk.offs_mclq as usize;
        read_mclq(data, mclq_offset).ok()
    } else {
        None
    };

    Ok(ChunkData { mcnk, mcvt, mclq })
}

/// Read MCVT chunk (height map)
fn read_mcvt(data: &[u8], offset: usize) -> Result<AdtMCVT> {
    if offset + 8 > data.len() {
        bail!("MCVT offset out of bounds");
    }

    let mut cursor = Cursor::new(&data[offset..]);
    let fourcc = read_fourcc(&mut cursor)?;
    if &fourcc != b"MCVT" {
        bail!("Expected MCVT chunk");
    }

    let _size = cursor.read_u32::<LittleEndian>()?;

    let mut heights = [0.0f32; AdtMCVT::NUM_HEIGHTS];
    for height in &mut heights {
        *height = cursor.read_f32::<LittleEndian>()?;
    }

    Ok(AdtMCVT { heights })
}

/// Read MCLQ chunk (old liquid format)
fn read_mclq(data: &[u8], offset: usize) -> Result<AdtMCLQ> {
    if offset + 8 > data.len() {
        bail!("MCLQ offset out of bounds");
    }

    let mut cursor = Cursor::new(&data[offset..]);
    let fourcc = read_fourcc(&mut cursor)?;
    if &fourcc != b"MCLQ" {
        bail!("Expected MCLQ chunk");
    }

    let _size = cursor.read_u32::<LittleEndian>()?;

    let height1 = cursor.read_f32::<LittleEndian>()?;
    let height2 = cursor.read_f32::<LittleEndian>()?;

    // Read liquid data grid (9x9)
    let mut liquid = [[LiquidData { light: 0, height: 0.0 }; ADT_CELL_SIZE + 1]; ADT_CELL_SIZE + 1];
    for y in 0..=ADT_CELL_SIZE {
        for x in 0..=ADT_CELL_SIZE {
            liquid[y][x] = LiquidData {
                light: cursor.read_u32::<LittleEndian>()?,
                height: cursor.read_f32::<LittleEndian>()?,
            };
        }
    }

    // Read liquid flags (8x8)
    let mut flags = [[0u8; ADT_CELL_SIZE]; ADT_CELL_SIZE];
    for y in 0..ADT_CELL_SIZE {
        for x in 0..ADT_CELL_SIZE {
            flags[y][x] = cursor.read_u8()?;
        }
    }

    // Read remaining data
    let mut data_bytes = [0u8; 84];
    cursor.read_exact(&mut data_bytes).unwrap_or(());

    Ok(AdtMCLQ {
        height1,
        height2,
        liquid,
        flags,
        data: data_bytes,
    })
}

/// Read MH2O chunk (new liquid format - WotLK+)
fn read_mh2o(data: &[u8], offset: usize) -> Result<AdtMH2O> {
    if offset + 8 > data.len() {
        bail!("MH2O offset out of bounds");
    }

    let mut cursor = Cursor::new(&data[offset..]);
    let fourcc = read_fourcc(&mut cursor)?;
    if &fourcc != b"MH2O" {
        bail!("Expected MH2O chunk");
    }

    let _size = cursor.read_u32::<LittleEndian>()?;

    let mut liquid = [[AdtLiquidInstance {
        offs_data: 0,
        used: 0,
        offs_attributes: 0,
    }; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID];

    for y in 0..ADT_CELLS_PER_GRID {
        for x in 0..ADT_CELLS_PER_GRID {
            liquid[y][x] = AdtLiquidInstance {
                offs_data: cursor.read_u32::<LittleEndian>()?,
                used: cursor.read_u32::<LittleEndian>()?,
                offs_attributes: cursor.read_u32::<LittleEndian>()?,
            };
        }
    }

    Ok(AdtMH2O { liquid })
}

/// Read string block (MMDX or MWMO chunk)
fn read_string_block(data: &[u8], offset: usize) -> Result<Vec<String>> {
    if offset + 8 > data.len() {
        bail!("String block offset out of bounds");
    }

    let mut cursor = Cursor::new(&data[offset..]);
    let fourcc = read_fourcc(&mut cursor)?;

    // Check for either MMDX or MWMO (or reversed: XDMM, OMWM)
    let fourcc_str = String::from_utf8_lossy(&fourcc);
    if &fourcc != b"MMDX" && &fourcc != b"MWMO" && fourcc_str != "XDMM" && fourcc_str != "OMWM" {
        bail!("Expected MMDX or MWMO chunk, got '{}'", fourcc_str);
    }

    let size = cursor.read_u32::<LittleEndian>()?;

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

/// Read MDDF chunk (M2 placements)
fn read_mddf(data: &[u8], offset: usize) -> Result<AdtMDDF> {
    if offset + 8 > data.len() {
        bail!("MDDF offset out of bounds");
    }

    let mut cursor = Cursor::new(&data[offset..]);
    let fourcc = read_fourcc(&mut cursor)?;
    let fourcc_str = String::from_utf8_lossy(&fourcc);
    if &fourcc != b"MDDF" && fourcc_str != "FDDM" {
        bail!("Expected MDDF chunk, got '{}'", fourcc_str);
    }

    let size = cursor.read_u32::<LittleEndian>()?;
    let entry_size = 36; // Size of M2Placement structure
    let n_entries = size / entry_size;

    let mut placements = Vec::with_capacity(n_entries as usize);

    for _ in 0..n_entries {
        placements.push(M2Placement {
            name_id: cursor.read_u32::<LittleEndian>()?,
            unique_id: cursor.read_u32::<LittleEndian>()?,
            position: [
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
            ],
            rotation: [
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
            ],
            scale: cursor.read_u16::<LittleEndian>()?,
            flags: cursor.read_u16::<LittleEndian>()?,
        });
    }

    Ok(AdtMDDF { placements })
}

/// Read MODF chunk (WMO placements)
fn read_modf(data: &[u8], offset: usize) -> Result<AdtMODF> {
    if offset + 8 > data.len() {
        bail!("MODF offset out of bounds");
    }

    let mut cursor = Cursor::new(&data[offset..]);
    let fourcc = read_fourcc(&mut cursor)?;
    let fourcc_str = String::from_utf8_lossy(&fourcc);
    if &fourcc != b"MODF" && fourcc_str != "FDOM" {
        bail!("Expected MODF chunk, got '{}'", fourcc_str);
    }

    let size = cursor.read_u32::<LittleEndian>()?;
    let entry_size = 64; // Size of WMOPlacement structure
    let n_entries = size / entry_size;

    let mut placements = Vec::with_capacity(n_entries as usize);

    for _ in 0..n_entries {
        placements.push(WMOPlacement {
            name_id: cursor.read_u32::<LittleEndian>()?,
            unique_id: cursor.read_u32::<LittleEndian>()?,
            position: [
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
            ],
            rotation: [
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
            ],
            bounding_box_min: [
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
            ],
            bounding_box_max: [
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
                cursor.read_f32::<LittleEndian>()?,
            ],
            flags: cursor.read_u16::<LittleEndian>()?,
            doodad_set: cursor.read_u16::<LittleEndian>()?,
            name_set: cursor.read_u16::<LittleEndian>()?,
            scale: cursor.read_u16::<LittleEndian>()?,
        });
    }

    Ok(AdtMODF { placements })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(ADT_GRID_SIZE, 128);
        assert_eq!(ADT_CELLS_PER_GRID, 16);
        assert_eq!(V9_SIZE, 129);
        assert_eq!(V8_SIZE, 128);
    }

    #[test]
    fn test_mcvt_size() {
        // 9*9 outer + 8*8 inner = 81 + 64 = 145
        assert_eq!(AdtMCVT::NUM_HEIGHTS, 145);
    }
}
