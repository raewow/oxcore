//! Directory Binary File Writer
//!
//! Writes intermediate `dir_bin` file containing all model placements.
//! This file is consumed by the assembler phase to build final VMAP files.

use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use glam::Vec3;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::vmaps::types::BoundingBox;

/// Model spawn flags (matching MaNGOS vmapexport.h)
pub const MOD_M2: u32 = 1;
pub const MOD_WORLDSPAWN: u32 = 1 << 1;
pub const MOD_HAS_BOUND: u32 = 1 << 2;

/// Coordinate transformation (fixCoords)
/// Converts from WoW coordinates to VMAP coordinates
/// Matches MaNGOS: Vec3D(v.z, v.x, v.y) - cyclic rotation
#[inline]
pub fn fix_coords(pos: Vec3) -> Vec3 {
    Vec3::new(pos.z, pos.x, pos.y)
}

/// Entry to write to dir_bin file
#[derive(Debug, Clone)]
pub struct DirBinEntry {
    pub map_id: u32,
    pub tile_x: u32,
    pub tile_y: u32,
    pub flags: u32,
    pub adt_id: u16,
    pub unique_id: u32,
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: f32,
    /// Bounds are only written/read when flags & MOD_HAS_BOUND != 0
    pub bounds: Option<BoundingBox>,
    pub name: String,
}

/// Thread-safe dir_bin writer
pub struct DirBinWriter {
    writer: Arc<Mutex<BufWriter<File>>>,
}

impl DirBinWriter {
    /// Create a new dir_bin writer
    /// Opens file in append mode to allow concurrent writes from multiple threads
    pub fn new(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("Failed to create dir_bin file: {}", path.display()))?;

        Ok(Self {
            writer: Arc::new(Mutex::new(BufWriter::new(file))),
        })
    }

    /// Write a single entry to dir_bin
    ///
    /// MaNGOS dir_bin format:
    /// mapID (u32), tileX (u32), tileY (u32), flags (u32), adtId (u16),
    /// uniqueId (u32), position (3x f32), rotation (3x f32), scale (f32),
    /// [bounds (6x f32) - only if flags & MOD_HAS_BOUND],
    /// nameLen (u32), name (char[])
    pub fn write_entry(&self, entry: &DirBinEntry) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();

        // Apply coordinate transformation
        let position = fix_coords(entry.position);

        writer.write_u32::<LittleEndian>(entry.map_id)?;
        writer.write_u32::<LittleEndian>(entry.tile_x)?;
        writer.write_u32::<LittleEndian>(entry.tile_y)?;
        writer.write_u32::<LittleEndian>(entry.flags)?;
        writer.write_u16::<LittleEndian>(entry.adt_id)?;
        writer.write_u32::<LittleEndian>(entry.unique_id)?;

        // Position (transformed)
        writer.write_f32::<LittleEndian>(position.x)?;
        writer.write_f32::<LittleEndian>(position.y)?;
        writer.write_f32::<LittleEndian>(position.z)?;

        // Rotation (as-is, in radians)
        writer.write_f32::<LittleEndian>(entry.rotation.x)?;
        writer.write_f32::<LittleEndian>(entry.rotation.y)?;
        writer.write_f32::<LittleEndian>(entry.rotation.z)?;

        // Scale
        writer.write_f32::<LittleEndian>(entry.scale)?;

        // Bounds - only written when MOD_HAS_BOUND flag is set (WMO entries)
        // M2 entries (MOD_M2) do NOT have bounds in MaNGOS dir_bin format
        if entry.flags & MOD_HAS_BOUND != 0 {
            if let Some(ref bounds) = entry.bounds {
                let bounds_min = fix_coords(bounds.min);
                let bounds_max = fix_coords(bounds.max);
                writer.write_f32::<LittleEndian>(bounds_min.x)?;
                writer.write_f32::<LittleEndian>(bounds_min.y)?;
                writer.write_f32::<LittleEndian>(bounds_min.z)?;
                writer.write_f32::<LittleEndian>(bounds_max.x)?;
                writer.write_f32::<LittleEndian>(bounds_max.y)?;
                writer.write_f32::<LittleEndian>(bounds_max.z)?;
            } else {
                // Flag set but no bounds - write zeros
                for _ in 0..6 {
                    writer.write_f32::<LittleEndian>(0.0)?;
                }
            }
        }

        // Name (length-prefixed)
        let name_len = entry.name.len() as u32;
        writer.write_u32::<LittleEndian>(name_len)?;
        writer.write_all(entry.name.as_bytes())?;

        Ok(())
    }

    /// Clone the writer (shares same underlying file)
    pub fn clone(&self) -> Self {
        Self {
            writer: Arc::clone(&self.writer),
        }
    }

    /// Flush writes to disk
    pub fn flush(&self) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.flush().context("Failed to flush dir_bin file")?;
        Ok(())
    }
}

/// Unique object ID generator (thread-safe)
pub struct UniqueIdGenerator {
    next_id: Arc<Mutex<u32>>,
}

impl UniqueIdGenerator {
    /// Create a new ID generator starting at 1
    pub fn new() -> Self {
        Self {
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Generate a unique ID
    /// In MaNGOS, this uses a map<pair<clientId, doodadId>, uniqueId>
    /// For simplicity, we just use a counter
    pub fn generate(&self) -> u32 {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        id
    }

    /// Clone the generator (shares same counter)
    pub fn clone(&self) -> Self {
        Self {
            next_id: Arc::clone(&self.next_id),
        }
    }
}

impl Default for UniqueIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Reader for dir_bin files
pub struct DirBinReader;

impl DirBinReader {
    /// Read all entries from a dir_bin file
    pub fn read_all(path: &std::path::Path) -> Result<Vec<DirBinEntry>> {

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();

        loop {
            // Try to read the next entry
            let entry = match read_entry(&mut reader) {
                Ok(e) => e,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            };
            entries.push(entry);
        }

        Ok(entries)
    }
}

fn read_entry<R: Read>(reader: &mut R) -> std::io::Result<DirBinEntry> {
    let map_id = reader.read_u32::<LittleEndian>()?;
    let tile_x = reader.read_u32::<LittleEndian>()?;
    let tile_y = reader.read_u32::<LittleEndian>()?;
    let flags = reader.read_u32::<LittleEndian>()?;
    let adt_id = reader.read_u16::<LittleEndian>()?;
    let unique_id = reader.read_u32::<LittleEndian>()?;

    // Position
    let pos_x = reader.read_f32::<LittleEndian>()?;
    let pos_y = reader.read_f32::<LittleEndian>()?;
    let pos_z = reader.read_f32::<LittleEndian>()?;
    let position = Vec3::new(pos_x, pos_y, pos_z);

    // Rotation
    let rot_x = reader.read_f32::<LittleEndian>()?;
    let rot_y = reader.read_f32::<LittleEndian>()?;
    let rot_z = reader.read_f32::<LittleEndian>()?;
    let rotation = Vec3::new(rot_x, rot_y, rot_z);

    let scale = reader.read_f32::<LittleEndian>()?;

    // Bounding box - only present when MOD_HAS_BOUND flag is set
    let bounds = if flags & MOD_HAS_BOUND != 0 {
        let min_x = reader.read_f32::<LittleEndian>()?;
        let min_y = reader.read_f32::<LittleEndian>()?;
        let min_z = reader.read_f32::<LittleEndian>()?;
        let max_x = reader.read_f32::<LittleEndian>()?;
        let max_y = reader.read_f32::<LittleEndian>()?;
        let max_z = reader.read_f32::<LittleEndian>()?;
        Some(BoundingBox {
            min: Vec3::new(min_x, min_y, min_z),
            max: Vec3::new(max_x, max_y, max_z),
        })
    } else {
        None
    };

    // Name (length-prefixed)
    let name_length = reader.read_u32::<LittleEndian>()?;
    let mut name_bytes = vec![0u8; name_length as usize];
    reader.read_exact(&mut name_bytes)?;
    let name = String::from_utf8_lossy(&name_bytes).to_string();

    Ok(DirBinEntry {
        map_id,
        tile_x,
        tile_y,
        flags,
        adt_id,
        unique_id,
        position,
        rotation,
        scale,
        bounds,
        name,
    })
}
