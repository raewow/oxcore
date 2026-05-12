//! DBC (Database Client) File Loader
//! Format:
//! - Header: 4 bytes magic "WDBC" (0x43424457)
//! - Record count: u32
//! - Field count: u32
//! - Record size: u32
//! - String size: u32
//! - Data: record data + string table

use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::Read;

const DBC_MAGIC: u32 = 0x43424457; // "WDBC" in little-endian

/// Field format types for DBC files
#[derive(Debug, Clone, Copy)]
pub enum FieldFormat {
    /// Ignore/default, 4 bytes
    Na,
    /// Ignore/default, 1 byte
    NaByte,
    /// String (char*)
    String,
    /// Float (f32)
    Float,
    /// Integer (u32)
    Int,
    /// Byte (u8)
    Byte,
    /// 64-bit integer (u64)
    Int64,
}

impl FieldFormat {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'x' => Some(FieldFormat::Na),
            'X' => Some(FieldFormat::NaByte),
            's' => Some(FieldFormat::String),
            'f' => Some(FieldFormat::Float),
            'i' => Some(FieldFormat::Int),
            'b' => Some(FieldFormat::Byte),
            'L' => Some(FieldFormat::Int64),
            _ => None,
        }
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        match self {
            FieldFormat::Na | FieldFormat::String | FieldFormat::Float | FieldFormat::Int => 4,
            FieldFormat::NaByte | FieldFormat::Byte => 1,
            FieldFormat::Int64 => 8,
        }
    }
}

pub struct DbcFileLoader {
    record_count: u32,
    field_count: u32,
    record_size: u32,
    string_size: u32,
    field_offsets: Vec<u32>,
    data: Vec<u8>,
    string_table: Vec<u8>,
}

impl DbcFileLoader {
    pub fn new() -> Self {
        Self {
            record_count: 0,
            field_count: 0,
            record_size: 0,
            string_size: 0,
            field_offsets: Vec::new(),
            data: Vec::new(),
            string_table: Vec::new(),
        }
    }

    pub fn load(&mut self, filename: &str, format: &str) -> Result<()> {
        let mut file = File::open(filename)
            .with_context(|| format!("Failed to open DBC file: {}", filename))?;

        let magic = file
            .read_u32::<LittleEndian>()
            .context("Failed to read DBC magic header")?;

        if magic != DBC_MAGIC {
            anyhow::bail!(
                "Invalid DBC file magic: expected 0x{:08X}, got 0x{:08X}",
                DBC_MAGIC,
                magic
            );
        }

        let record_count = file
            .read_u32::<LittleEndian>()
            .context("Failed to read record count")?;
        let field_count = file
            .read_u32::<LittleEndian>()
            .context("Failed to read field count")?;
        let record_size = file
            .read_u32::<LittleEndian>()
            .context("Failed to read record size")?;
        let string_size = file
            .read_u32::<LittleEndian>()
            .context("Failed to read string size")?;

        let mut field_offsets = Vec::with_capacity(field_count as usize);
        field_offsets.push(0);

        let format_chars: Vec<char> = format.chars().collect();
        for i in 1..field_count as usize {
            let prev_offset = field_offsets[i - 1];
            let format_char = format_chars.get(i - 1).copied().unwrap_or('x');
            let field_format = FieldFormat::from_char(format_char).unwrap_or(FieldFormat::Na);
            field_offsets.push(prev_offset + field_format.size() as u32);
        }

        let data_size = (record_size * record_count + string_size) as usize;
        let mut data = vec![0u8; data_size];
        file.read_exact(&mut data)
            .context("Failed to read DBC data")?;

        let record_data_size = (record_size * record_count) as usize;
        let string_table = data.split_off(record_data_size);

        self.record_count = record_count;
        self.field_count = field_count;
        self.record_size = record_size;
        self.string_size = string_size;
        self.field_offsets = field_offsets;
        self.data = data;
        self.string_table = string_table;

        Ok(())
    }

    pub fn record_count(&self) -> u32 {
        self.record_count
    }

    pub fn field_count(&self) -> u32 {
        self.field_count
    }

    pub fn field_offset(&self, field: usize) -> Option<u32> {
        self.field_offsets.get(field).copied()
    }

    pub fn get_record(&self, index: usize) -> Option<DbcRecord<'_>> {
        if index >= self.record_count as usize {
            return None;
        }

        let offset = (index * self.record_size as usize) as usize;
        if offset + self.record_size as usize > self.data.len() {
            return None;
        }

        Some(DbcRecord {
            loader: self,
            offset,
        })
    }

    pub fn get_string(&self, offset: u32) -> Option<&str> {
        if offset as usize >= self.string_table.len() {
            return None;
        }

        // Find null terminator
        let start = offset as usize;
        let end = self.string_table[start..]
            .iter()
            .position(|&b| b == 0)
            .map(|pos| start + pos)
            .unwrap_or(self.string_table.len());

        std::str::from_utf8(&self.string_table[start..end]).ok()
    }
}

pub struct DbcRecord<'a> {
    loader: &'a DbcFileLoader,
    offset: usize,
}

impl<'a> DbcRecord<'a> {
    pub fn field_count(&self) -> u32 {
        self.loader.field_count()
    }

    pub fn get_u32(&self, field: usize) -> Option<u32> {
        let field_offset = self.loader.field_offset(field)?;
        let byte_offset = self.offset + field_offset as usize;

        if byte_offset + 4 > self.loader.data.len() {
            return None;
        }

        let bytes = &self.loader.data[byte_offset..byte_offset + 4];
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn get_i32(&self, field: usize) -> Option<i32> {
        let field_offset = self.loader.field_offset(field)?;
        let byte_offset = self.offset + field_offset as usize;

        if byte_offset + 4 > self.loader.data.len() {
            return None;
        }

        let bytes = &self.loader.data[byte_offset..byte_offset + 4];
        Some(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn get_u8(&self, field: usize) -> Option<u8> {
        let field_offset = self.loader.field_offset(field)?;
        let byte_offset = self.offset + field_offset as usize;

        if byte_offset >= self.loader.data.len() {
            return None;
        }

        Some(self.loader.data[byte_offset])
    }

    pub fn get_f32(&self, field: usize) -> Option<f32> {
        let field_offset = self.loader.field_offset(field)?;
        let byte_offset = self.offset + field_offset as usize;

        if byte_offset + 4 > self.loader.data.len() {
            return None;
        }

        let bytes = &self.loader.data[byte_offset..byte_offset + 4];
        Some(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn get_string(&self, field: usize) -> Option<&'a str> {
        let string_offset = self.get_u32(field)?;
        self.loader.get_string(string_offset)
    }

    pub fn get_bytes(&self, field: usize, size: usize) -> Option<&'a [u8]> {
        let field_offset = self.loader.field_offset(field)?;
        let byte_offset = self.offset + field_offset as usize;

        if byte_offset + size > self.loader.data.len() {
            return None;
        }

        Some(&self.loader.data[byte_offset..byte_offset + size])
    }
}
