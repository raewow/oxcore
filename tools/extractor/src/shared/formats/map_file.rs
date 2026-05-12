//! Map File Format
//!
//! Custom binary format for storing extracted map data.
//! This format is optimized for fast loading by the game server.

use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

use crate::shared::formats::adt::{ADT_CELLS_PER_GRID, ADT_GRID_SIZE, V8_SIZE, V9_SIZE};

// File magic constants
pub const MAP_MAGIC: &[u8; 4] = b"MAPS";
pub const MAP_VERSION_MAGIC: &[u8; 4] = b"z1.4";
pub const MAP_AREA_MAGIC: &[u8; 4] = b"AREA";
pub const MAP_HEIGHT_MAGIC: &[u8; 4] = b"MHGT";
pub const MAP_LIQUID_MAGIC: &[u8; 4] = b"MLIQ";

// Header flags - area (u16 flags field)
pub const MAP_AREA_NO_AREA: u16 = 0x0001;

// Header flags - height (u32 flags field)
pub const MAP_HEIGHT_NO_HEIGHT: u32 = 0x0001;
pub const MAP_HEIGHT_AS_INT16: u32 = 0x0002;
pub const MAP_HEIGHT_AS_INT8: u32 = 0x0004;

// Header flags - liquid (u8 flags field)
pub const MAP_LIQUID_NO_TYPE: u8 = 0x01;
pub const MAP_LIQUID_NO_HEIGHT: u8 = 0x02;

// Liquid type flags (matches MaNGOS GridMapDefines.h)
// These are left-shifted for flag usage in DBC
pub const MAP_LIQUID_TYPE_NO_WATER: u8 = 0x00;
pub const MAP_LIQUID_TYPE_MAGMA: u8 = 0x01;
pub const MAP_LIQUID_TYPE_OCEAN: u8 = 0x02;
pub const MAP_LIQUID_TYPE_SLIME: u8 = 0x04;
pub const MAP_LIQUID_TYPE_WATER: u8 = 0x08;
pub const MAP_LIQUID_TYPE_DEEP_WATER: u8 = 0x10;

/// Main file header
#[derive(Debug, Clone)]
pub struct GridMapFileHeader {
    pub map_magic: u32,           // "MAPS"
    pub version_magic: u32,       // "z1.4"
    pub area_map_offset: u32,
    pub area_map_size: u32,
    pub height_map_offset: u32,
    pub height_map_size: u32,
    pub liquid_map_offset: u32,
    pub liquid_map_size: u32,
    pub holes_offset: u32,
    pub holes_size: u32,
}

impl GridMapFileHeader {
    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(MAP_MAGIC)?;
        writer.write_all(MAP_VERSION_MAGIC)?;
        writer.write_u32::<LittleEndian>(self.area_map_offset)?;
        writer.write_u32::<LittleEndian>(self.area_map_size)?;
        writer.write_u32::<LittleEndian>(self.height_map_offset)?;
        writer.write_u32::<LittleEndian>(self.height_map_size)?;
        writer.write_u32::<LittleEndian>(self.liquid_map_offset)?;
        writer.write_u32::<LittleEndian>(self.liquid_map_size)?;
        writer.write_u32::<LittleEndian>(self.holes_offset)?;
        writer.write_u32::<LittleEndian>(self.holes_size)?;
        Ok(())
    }
}

/// Area map header
/// Matches MaNGOS GridMapAreaHeader: fourcc(4) + flags(2) + gridArea(2) = 8 bytes
#[derive(Debug, Clone)]
pub struct GridMapAreaHeader {
    pub fourcc: u32,              // "AREA"
    pub flags: u16,               // Area flags (MAP_AREA_NO_AREA if uniform)
    pub grid_area: u16,           // Single area ID if uniform
}

impl GridMapAreaHeader {
    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(MAP_AREA_MAGIC)?;
        writer.write_u16::<LittleEndian>(self.flags)?;
        writer.write_u16::<LittleEndian>(self.grid_area)?;
        Ok(())
    }
}

/// Height map header
#[derive(Debug, Clone)]
pub struct GridMapHeightHeader {
    pub fourcc: u32,              // "MHGT"
    pub flags: u32,
    pub grid_height: f32,         // Minimum height
    pub grid_max_height: f32,     // Maximum height
}

impl GridMapHeightHeader {
    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(MAP_HEIGHT_MAGIC)?;
        writer.write_u32::<LittleEndian>(self.flags)?;
        writer.write_f32::<LittleEndian>(self.grid_height)?;
        writer.write_f32::<LittleEndian>(self.grid_max_height)?;
        Ok(())
    }
}

/// Liquid map header
/// Matches MaNGOS GridMapLiquidHeader: fourcc(4) + flags(1) + liquidFlags(1) + liquidType(2) +
/// offsetX(1) + offsetY(1) + width(1) + height(1) + liquidLevel(4) = 16 bytes
#[derive(Debug, Clone)]
pub struct GridMapLiquidHeader {
    pub fourcc: u32,              // "MLIQ"
    pub flags: u8,                // MAP_LIQUID_NO_TYPE, MAP_LIQUID_NO_HEIGHT
    pub liquid_flags: u8,         // Liquid type flags (water/ocean/magma/slime)
    pub liquid_type: u16,         // Liquid entry ID
    pub offset_x: u8,
    pub offset_y: u8,
    pub width: u8,
    pub height: u8,
    pub liquid_level: f32,
}

impl GridMapLiquidHeader {
    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(MAP_LIQUID_MAGIC)?;
        writer.write_u8(self.flags)?;
        writer.write_u8(self.liquid_flags)?;
        writer.write_u16::<LittleEndian>(self.liquid_type)?;
        writer.write_u8(self.offset_x)?;
        writer.write_u8(self.offset_y)?;
        writer.write_u8(self.width)?;
        writer.write_u8(self.height)?;
        writer.write_f32::<LittleEndian>(self.liquid_level)?;
        Ok(())
    }
}

/// Height data (can be stored in different formats for compression)
/// Uses Box to keep large arrays on heap and avoid stack overflow
#[derive(Debug, Clone)]
pub enum HeightData {
    None,
    Float {
        v9: Box<[[f32; V9_SIZE]; V9_SIZE]>,
        v8: Box<[[f32; V8_SIZE]; V8_SIZE]>,
    },
    UInt16 {
        v9: Box<[[u16; V9_SIZE]; V9_SIZE]>,
        v8: Box<[[u16; V8_SIZE]; V8_SIZE]>,
    },
    UInt8 {
        v9: Box<[[u8; V9_SIZE]; V9_SIZE]>,
        v8: Box<[[u8; V8_SIZE]; V8_SIZE]>,
    },
}

impl HeightData {
    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            HeightData::None => Ok(()),
            HeightData::Float { v9, v8 } => {
                // Write V9 grid (129x129)
                for y in 0..V9_SIZE {
                    for x in 0..V9_SIZE {
                        writer.write_f32::<LittleEndian>(v9[y][x])?;
                    }
                }
                // Write V8 grid (128x128)
                for y in 0..V8_SIZE {
                    for x in 0..V8_SIZE {
                        writer.write_f32::<LittleEndian>(v8[y][x])?;
                    }
                }
                Ok(())
            }
            HeightData::UInt16 { v9, v8 } => {
                // Write V9 grid
                for y in 0..V9_SIZE {
                    for x in 0..V9_SIZE {
                        writer.write_u16::<LittleEndian>(v9[y][x])?;
                    }
                }
                // Write V8 grid
                for y in 0..V8_SIZE {
                    for x in 0..V8_SIZE {
                        writer.write_u16::<LittleEndian>(v8[y][x])?;
                    }
                }
                Ok(())
            }
            HeightData::UInt8 { v9, v8 } => {
                // Write V9 grid
                for y in 0..V9_SIZE {
                    for x in 0..V9_SIZE {
                        writer.write_u8(v9[y][x])?;
                    }
                }
                // Write V8 grid
                for y in 0..V8_SIZE {
                    for x in 0..V8_SIZE {
                        writer.write_u8(v8[y][x])?;
                    }
                }
                Ok(())
            }
        }
    }

    /// Calculate the size in bytes
    pub fn size(&self) -> usize {
        match self {
            HeightData::None => 0,
            HeightData::Float { .. } => {
                (V9_SIZE * V9_SIZE + V8_SIZE * V8_SIZE) * 4 // 4 bytes per f32
            }
            HeightData::UInt16 { .. } => {
                (V9_SIZE * V9_SIZE + V8_SIZE * V8_SIZE) * 2 // 2 bytes per u16
            }
            HeightData::UInt8 { .. } => {
                V9_SIZE * V9_SIZE + V8_SIZE * V8_SIZE // 1 byte per u8
            }
        }
    }
}

/// Area data
#[derive(Debug, Clone)]
pub enum AreaData {
    Single(u16),
    Grid([[u16; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID]),
}

impl AreaData {
    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            AreaData::Single(_) => Ok(()), // Already in header
            AreaData::Grid(grid) => {
                for y in 0..ADT_CELLS_PER_GRID {
                    for x in 0..ADT_CELLS_PER_GRID {
                        writer.write_u16::<LittleEndian>(grid[y][x])?;
                    }
                }
                Ok(())
            }
        }
    }

    pub fn size(&self) -> usize {
        match self {
            AreaData::Single(_) => 0,
            AreaData::Grid(_) => ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID * 2, // 2 bytes per u16
        }
    }
}

/// Liquid data
#[derive(Debug, Clone)]
pub struct LiquidData {
    pub entry: [[u16; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID],
    pub flags: [[u8; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID],
    pub height: Option<[[f32; ADT_GRID_SIZE + 1]; ADT_GRID_SIZE + 1]>,
}

impl LiquidData {
    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Write liquid entries
        for y in 0..ADT_CELLS_PER_GRID {
            for x in 0..ADT_CELLS_PER_GRID {
                writer.write_u16::<LittleEndian>(self.entry[y][x])?;
            }
        }

        // Write liquid flags
        for y in 0..ADT_CELLS_PER_GRID {
            for x in 0..ADT_CELLS_PER_GRID {
                writer.write_u8(self.flags[y][x])?;
            }
        }

        // Write height map if present
        if let Some(heights) = &self.height {
            for y in 0..(ADT_GRID_SIZE + 1) {
                for x in 0..(ADT_GRID_SIZE + 1) {
                    writer.write_f32::<LittleEndian>(heights[y][x])?;
                }
            }
        }

        Ok(())
    }

    pub fn size(&self) -> usize {
        let base_size = (ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID * 2) + // entry (u16)
                        (ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID);      // flags (u8)

        if self.height.is_some() {
            base_size + (ADT_GRID_SIZE + 1) * (ADT_GRID_SIZE + 1) * 4 // f32
        } else {
            base_size
        }
    }
}

/// Holes data
pub type HolesData = [[u16; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID];

pub fn write_holes<W: Write>(writer: &mut W, holes: &HolesData) -> std::io::Result<()> {
    for y in 0..ADT_CELLS_PER_GRID {
        for x in 0..ADT_CELLS_PER_GRID {
            writer.write_u16::<LittleEndian>(holes[y][x])?;
        }
    }
    Ok(())
}

pub fn holes_size() -> usize {
    ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID * 2 // 2 bytes per u16
}
