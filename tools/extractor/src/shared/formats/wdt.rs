//! WDT File Parser
//!
//! WDT (World Data Table) files define which ADT tiles exist for a given map.
//! They contain a 64x64 grid indicating which tiles have terrain data.

use anyhow::{bail, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read, Seek};
use std::path::Path;

/// WDT grid size (64x64 tiles)
pub const WDT_MAP_SIZE: usize = 64;

/// WDT main header (MPHD chunk)
#[derive(Debug, Clone)]
pub struct WdtMPHD {
    pub flags: u32,
    pub something: u32,
    pub unused: [u32; 6],
}

/// Tile flags in MAIN chunk
#[derive(Debug, Clone, Copy)]
pub struct WdtMAINEntry {
    pub flags: u32,
    pub async_id: u32,
}

impl WdtMAINEntry {
    /// Check if this tile has ADT data
    pub fn has_adt(&self) -> bool {
        self.flags & 0x1 != 0
    }

    /// Check if this tile is loaded
    pub fn is_loaded(&self) -> bool {
        self.flags & 0x2 != 0
    }
}

/// MODF structure - WMO instance placement (64 bytes)
#[derive(Debug, Clone)]
pub struct WdtMODF {
    pub id: u32,           // WMO name index
    pub unique_id: u32,    // Unique instance ID
    pub position: [f32; 3], // X, Y, Z
    pub rotation: [f32; 3], // Rotation
    pub bounds_min: [f32; 3], // Bounding box min
    pub bounds_max: [f32; 3], // Bounding box max
    pub flags: u16,
    pub doodad_set: u16,
    pub name_set: u16,
    pub scale: u16,
}

/// WDT file structure
pub struct WDTFile {
    pub name: String,
    pub header: WdtMPHD,
    pub tiles: [[WdtMAINEntry; WDT_MAP_SIZE]; WDT_MAP_SIZE],
    pub wmo_names: Vec<String>,     // MWMO chunk - global WMO filenames
    pub wmo_placements: Vec<WdtMODF>, // MODF chunk - global WMO placements
}

impl WDTFile {
    /// Load a WDT file from disk
    pub fn load_file(path: &Path) -> Result<Self> {
        let name = path.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read WDT file: {}", path.display()))?;

        Self::from_bytes(name, &data)
    }

    /// Parse a WDT file from bytes
    pub fn from_bytes(name: String, data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);

        // Some WDT files may have different chunk ordering or formats
        // Try to find MVER, MPHD, MAIN, MWMO, and MODF chunks in any order
        let mut mver_version = None;
        let mut header = None;
        let mut tiles = None;
        let mut wmo_names = Vec::new();
        let mut wmo_placements = Vec::new();

        // Read chunks until we find what we need
        while cursor.position() < data.len() as u64 {
            let pos = cursor.position();

            // Check if we have enough bytes for a chunk header (8 bytes)
            if data.len() as u64 - pos < 8 {
                break;
            }

            let (fourcc, size) = match read_chunk_header(&mut cursor) {
                Ok(h) => h,
                Err(_) => break, // End of file or invalid chunk
            };

            // Handle different chunk types (check both normal and reversed byte order)
            let fourcc_str = String::from_utf8_lossy(&fourcc);
            let is_mver = &fourcc == b"MVER" || fourcc_str == "REVM";
            let is_mphd = &fourcc == b"MPHD" || fourcc_str == "DHPM";
            let is_main = &fourcc == b"MAIN" || fourcc_str == "NIAM";
            let is_mwmo = &fourcc == b"MWMO" || fourcc_str == "OMWM";
            let is_modf = &fourcc == b"MODF" || fourcc_str == "FDOM";

            if is_mver {
                if size == 4 {
                    mver_version = Some(cursor.read_u32::<LittleEndian>()
                        .context("Failed to read MVER version")?);
                } else {
                    // Skip invalid size
                    cursor.seek(std::io::SeekFrom::Current(size as i64))?;
                }
            } else if is_mphd {
                if size >= 32 {
                    cursor.seek(std::io::SeekFrom::Current(-8))?; // Go back to read chunk
                    header = Some(read_mphd(&mut cursor)?);
                } else {
                    cursor.seek(std::io::SeekFrom::Current(size as i64))?;
                }
            } else if is_main {
                let expected_size = (WDT_MAP_SIZE * WDT_MAP_SIZE * 8) as u32;
                if size == expected_size {
                    cursor.seek(std::io::SeekFrom::Current(-8))?; // Go back to read chunk
                    tiles = Some(read_main(&mut cursor)?);
                } else {
                    cursor.seek(std::io::SeekFrom::Current(size as i64))?;
                }
            } else if is_mwmo {
                if size > 0 {
                    cursor.seek(std::io::SeekFrom::Current(-8))?; // Go back to read chunk
                    wmo_names = read_mwmo(&mut cursor, size)?;
                }
            } else if is_modf {
                if size > 0 && size % 64 == 0 {
                    cursor.seek(std::io::SeekFrom::Current(-8))?; // Go back to read chunk
                    wmo_placements = read_modf(&mut cursor, size)?;
                } else {
                    cursor.seek(std::io::SeekFrom::Current(size as i64))?;
                }
            } else {
                // Skip unknown chunks (including REVM which might be MVER in wrong byte order)
                cursor.seek(std::io::SeekFrom::Current(size as i64))?;
            }
        }

        // Validate we found required chunks
        let header = header.ok_or_else(|| anyhow::anyhow!("MPHD chunk not found in WDT file"))?;
        let tiles = tiles.ok_or_else(|| anyhow::anyhow!("MAIN chunk not found in WDT file"))?;

        // MVER is optional in some formats, but warn if version is unexpected
        if let Some(version) = mver_version {
            if version != 18 {
                tracing::warn!("WDT file has version {} (expected 18), continuing anyway", version);
            }
        }

        Ok(Self {
            name,
            header,
            tiles,
            wmo_names,
            wmo_placements,
        })
    }

    /// Check if a specific tile exists
    pub fn tile_exists(&self, x: usize, y: usize) -> bool {
        if x >= WDT_MAP_SIZE || y >= WDT_MAP_SIZE {
            return false;
        }
        self.tiles[y][x].has_adt()
    }

    /// Get all existing tile coordinates
    pub fn get_existing_tiles(&self) -> Vec<(usize, usize)> {
        let mut tiles = Vec::new();
        for y in 0..WDT_MAP_SIZE {
            for x in 0..WDT_MAP_SIZE {
                if self.tiles[y][x].has_adt() {
                    tiles.push((x, y));
                }
            }
        }
        tiles
    }

    /// Count the number of existing tiles
    pub fn tile_count(&self) -> usize {
        let mut count = 0;
        for y in 0..WDT_MAP_SIZE {
            for x in 0..WDT_MAP_SIZE {
                if self.tiles[y][x].has_adt() {
                    count += 1;
                }
            }
        }
        count
    }
}

/// Read a chunk header (fourcc + size)
fn read_chunk_header(cursor: &mut Cursor<&[u8]>) -> Result<([u8; 4], u32)> {
    let mut fourcc = [0u8; 4];
    cursor.read_exact(&mut fourcc)
        .context("Failed to read chunk fourcc")?;

    let size = cursor.read_u32::<LittleEndian>()
        .context("Failed to read chunk size")?;

    Ok((fourcc, size))
}

/// Read MVER chunk (version)
fn read_mver(cursor: &mut Cursor<&[u8]>) -> Result<u32> {
    let (fourcc, size) = read_chunk_header(cursor)?;

    if &fourcc != b"MVER" {
        bail!("Expected MVER chunk, got '{}'", String::from_utf8_lossy(&fourcc));
    }

    if size != 4 {
        bail!("Invalid MVER chunk size: {}", size);
    }

    cursor.read_u32::<LittleEndian>()
        .context("Failed to read version")
}

/// Read MPHD chunk (header)
fn read_mphd(cursor: &mut Cursor<&[u8]>) -> Result<WdtMPHD> {
    let (fourcc, size) = read_chunk_header(cursor)?;

    // Accept both MPHD and DHPM (reversed)
    let fourcc_str = String::from_utf8_lossy(&fourcc);
    if &fourcc != b"MPHD" && fourcc_str != "DHPM" {
        bail!("Expected MPHD chunk, got '{}'", fourcc_str);
    }

    if size < 32 {
        bail!("MPHD chunk too small: {}", size);
    }

    let flags = cursor.read_u32::<LittleEndian>()?;
    let something = cursor.read_u32::<LittleEndian>()?;

    let mut unused = [0u32; 6];
    for i in 0..6 {
        unused[i] = cursor.read_u32::<LittleEndian>()?;
    }

    Ok(WdtMPHD {
        flags,
        something,
        unused,
    })
}

/// Read MAIN chunk (tile flags)
fn read_main(cursor: &mut Cursor<&[u8]>) -> Result<[[WdtMAINEntry; WDT_MAP_SIZE]; WDT_MAP_SIZE]> {
    let (fourcc, size) = read_chunk_header(cursor)?;

    // Accept both MAIN and NIAM (reversed)
    let fourcc_str = String::from_utf8_lossy(&fourcc);
    if &fourcc != b"MAIN" && fourcc_str != "NIAM" {
        bail!("Expected MAIN chunk, got '{}'", fourcc_str);
    }

    let expected_size = (WDT_MAP_SIZE * WDT_MAP_SIZE * 8) as u32; // 8 bytes per entry
    if size != expected_size {
        bail!("Invalid MAIN chunk size: {} (expected {})", size, expected_size);
    }

    let mut tiles = [[WdtMAINEntry { flags: 0, async_id: 0 }; WDT_MAP_SIZE]; WDT_MAP_SIZE];

    for y in 0..WDT_MAP_SIZE {
        for x in 0..WDT_MAP_SIZE {
            tiles[y][x] = WdtMAINEntry {
                flags: cursor.read_u32::<LittleEndian>()?,
                async_id: cursor.read_u32::<LittleEndian>()?,
            };
        }
    }

    Ok(tiles)
}

/// Read MWMO chunk (global WMO names)
fn read_mwmo(cursor: &mut Cursor<&[u8]>, chunk_size: u32) -> Result<Vec<String>> {
    let (fourcc, size) = read_chunk_header(cursor)?;

    // Accept both MWMO and OMWM (reversed)
    let fourcc_str = String::from_utf8_lossy(&fourcc);
    if &fourcc != b"MWMO" && fourcc_str != "OMWM" {
        bail!("Expected MWMO chunk, got '{}'", fourcc_str);
    }

    if size == 0 {
        return Ok(Vec::new());
    }

    // Read all bytes
    let mut buffer = vec![0u8; size as usize];
    cursor.read_exact(&mut buffer)
        .context("Failed to read MWMO chunk data")?;

    // Parse null-terminated strings
    let mut names = Vec::new();
    let mut start = 0;

    for i in 0..buffer.len() {
        if buffer[i] == 0 {
            if i > start {
                let name_bytes = &buffer[start..i];
                if let Ok(name) = String::from_utf8(name_bytes.to_vec()) {
                    // Keep full path for WMO files
                    names.push(name);
                }
            }
            start = i + 1;
        }
    }

    Ok(names)
}

/// Read MODF chunk (global WMO placements)
fn read_modf(cursor: &mut Cursor<&[u8]>, chunk_size: u32) -> Result<Vec<WdtMODF>> {
    let (fourcc, size) = read_chunk_header(cursor)?;

    // Accept both MODF and FDOM (reversed)
    let fourcc_str = String::from_utf8_lossy(&fourcc);
    if &fourcc != b"MODF" && fourcc_str != "FDOM" {
        bail!("Expected MODF chunk, got '{}'", fourcc_str);
    }

    if size == 0 || size % 64 != 0 {
        bail!("Invalid MODF chunk size: {} (must be multiple of 64)", size);
    }

    let count = size / 64;
    let mut placements = Vec::with_capacity(count as usize);

    for _ in 0..count {
        let id = cursor.read_u32::<LittleEndian>()?;
        let unique_id = cursor.read_u32::<LittleEndian>()?;

        let mut position = [0f32; 3];
        for i in 0..3 {
            position[i] = cursor.read_f32::<LittleEndian>()?;
        }

        let mut rotation = [0f32; 3];
        for i in 0..3 {
            rotation[i] = cursor.read_f32::<LittleEndian>()?;
        }

        let mut bounds_min = [0f32; 3];
        let mut bounds_max = [0f32; 3];
        for i in 0..3 {
            bounds_min[i] = cursor.read_f32::<LittleEndian>()?;
        }
        for i in 0..3 {
            bounds_max[i] = cursor.read_f32::<LittleEndian>()?;
        }

        let flags = cursor.read_u16::<LittleEndian>()?;
        let doodad_set = cursor.read_u16::<LittleEndian>()?;
        let name_set = cursor.read_u16::<LittleEndian>()?;
        let scale = cursor.read_u16::<LittleEndian>()?;

        placements.push(WdtMODF {
            id,
            unique_id,
            position,
            rotation,
            bounds_min,
            bounds_max,
            flags,
            doodad_set,
            name_set,
            scale,
        });
    }

    Ok(placements)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wdt_constants() {
        assert_eq!(WDT_MAP_SIZE, 64);
    }

    #[test]
    fn test_entry_flags() {
        let entry = WdtMAINEntry {
            flags: 0x1,
            async_id: 0,
        };
        assert!(entry.has_adt());

        let entry2 = WdtMAINEntry {
            flags: 0x0,
            async_id: 0,
        };
        assert!(!entry2.has_adt());
    }
}
