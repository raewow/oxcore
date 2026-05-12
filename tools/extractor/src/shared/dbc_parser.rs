//! DBC File Parser
//!
//! Parser for World of Warcraft DBC (Database Client) files.
//!
//! DBC files are structured binary files that store game data in a table format.
//! Each file contains:
//! - Header (magic, record count, field count, record size, string table size)
//! - Record data (fixed-size records)
//! - String table (null-terminated strings)

use anyhow::{bail, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};
use std::path::Path;

/// DBC file magic bytes ("WDBC")
const DBC_MAGIC: &[u8; 4] = b"WDBC";

/// DBC file structure
pub struct DBCFile {
    filename: String,
    record_size: u32,
    record_count: u32,
    field_count: u32,
    string_size: u32,
    data: Vec<u8>,
    string_table: Vec<u8>,
}

/// A single record in a DBC file
pub struct DBCRecord<'a> {
    file: &'a DBCFile,
    offset: usize,
}

/// Iterator over DBC records
pub struct DBCIterator<'a> {
    file: &'a DBCFile,
    index: usize,
}

impl DBCFile {
    /// Open and parse a DBC file
    pub fn open(path: &Path) -> Result<Self> {
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Read the entire file
        let file_data = std::fs::read(path)
            .with_context(|| format!("Failed to read DBC file: {}", path.display()))?;

        Self::from_bytes(filename, &file_data)
    }

    /// Parse a DBC file from bytes
    pub fn from_bytes(filename: String, data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);

        // Read and validate header
        let mut magic = [0u8; 4];
        cursor.read_exact(&mut magic)
            .context("Failed to read DBC magic bytes")?;

        if &magic != DBC_MAGIC {
            bail!("Invalid DBC magic bytes. Expected 'WDBC', got '{}'",
                String::from_utf8_lossy(&magic));
        }

        let record_count = cursor.read_u32::<LittleEndian>()
            .context("Failed to read record count")?;
        let field_count = cursor.read_u32::<LittleEndian>()
            .context("Failed to read field count")?;
        let record_size = cursor.read_u32::<LittleEndian>()
            .context("Failed to read record size")?;
        let string_size = cursor.read_u32::<LittleEndian>()
            .context("Failed to read string size")?;

        // Validate header values
        if field_count * 4 != record_size {
            bail!("Invalid DBC header: field_count * 4 ({}) != record_size ({})",
                field_count * 4, record_size);
        }

        // Calculate data sizes
        let data_size = (record_count * record_size) as usize;

        // Read record data
        let mut record_data = vec![0u8; data_size];
        cursor.read_exact(&mut record_data)
            .context("Failed to read record data")?;

        // Read string table
        let mut string_table = vec![0u8; string_size as usize];
        cursor.read_exact(&mut string_table)
            .context("Failed to read string table")?;

        Ok(Self {
            filename,
            record_size,
            record_count,
            field_count,
            string_size,
            data: record_data,
            string_table,
        })
    }

    /// Get the number of records in this DBC file
    pub fn get_record_count(&self) -> usize {
        self.record_count as usize
    }

    /// Get the number of fields per record
    pub fn get_field_count(&self) -> usize {
        self.field_count as usize
    }

    /// Get the size of each record in bytes
    pub fn get_record_size(&self) -> usize {
        self.record_size as usize
    }

    /// Get a record by index
    pub fn get_record(&self, index: usize) -> Option<DBCRecord> {
        if index >= self.record_count as usize {
            return None;
        }

        Some(DBCRecord {
            file: self,
            offset: index * self.record_size as usize,
        })
    }

    /// Get the maximum ID value in the first field of all records
    pub fn get_max_id(&self) -> u32 {
        let mut max_id = 0u32;
        for i in 0..self.record_count {
            if let Some(record) = self.get_record(i as usize) {
                let id = record.get_uint(0);
                if id > max_id {
                    max_id = id;
                }
            }
        }
        max_id
    }

    /// Get an iterator over all records
    pub fn iter(&self) -> DBCIterator {
        DBCIterator {
            file: self,
            index: 0,
        }
    }

    /// Get the filename
    pub fn filename(&self) -> &str {
        &self.filename
    }
}

impl<'a> DBCRecord<'a> {
    /// Get an unsigned integer field
    pub fn get_uint(&self, field: usize) -> u32 {
        if field >= self.file.field_count as usize {
            return 0;
        }

        let offset = self.offset + field * 4;
        let mut cursor = Cursor::new(&self.file.data[offset..]);
        cursor.read_u32::<LittleEndian>().unwrap_or(0)
    }

    /// Get a signed integer field
    pub fn get_int(&self, field: usize) -> i32 {
        if field >= self.file.field_count as usize {
            return 0;
        }

        let offset = self.offset + field * 4;
        let mut cursor = Cursor::new(&self.file.data[offset..]);
        cursor.read_i32::<LittleEndian>().unwrap_or(0)
    }

    /// Get a float field
    pub fn get_float(&self, field: usize) -> f32 {
        if field >= self.file.field_count as usize {
            return 0.0;
        }

        let offset = self.offset + field * 4;
        let mut cursor = Cursor::new(&self.file.data[offset..]);
        cursor.read_f32::<LittleEndian>().unwrap_or(0.0)
    }

    /// Get a string field
    pub fn get_string(&self, field: usize) -> &str {
        if field >= self.file.field_count as usize {
            return "";
        }

        let offset = self.offset + field * 4;
        let mut cursor = Cursor::new(&self.file.data[offset..]);
        let string_offset = cursor.read_u32::<LittleEndian>().unwrap_or(0) as usize;

        if string_offset >= self.file.string_table.len() {
            return "";
        }

        // Find the null terminator
        let string_data = &self.file.string_table[string_offset..];
        let null_pos = string_data.iter()
            .position(|&c| c == 0)
            .unwrap_or(string_data.len());

        std::str::from_utf8(&string_data[..null_pos]).unwrap_or("")
    }
}

impl<'a> Iterator for DBCIterator<'a> {
    type Item = DBCRecord<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.file.record_count as usize {
            return None;
        }

        let record = self.file.get_record(self.index)?;
        self.index += 1;
        Some(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbc_validation() {
        // Create a minimal valid DBC file
        let mut data = Vec::new();

        // Header
        data.extend_from_slice(b"WDBC"); // Magic
        data.extend_from_slice(&1u32.to_le_bytes()); // record_count
        data.extend_from_slice(&2u32.to_le_bytes()); // field_count
        data.extend_from_slice(&8u32.to_le_bytes()); // record_size (2 fields * 4 bytes)
        data.extend_from_slice(&1u32.to_le_bytes()); // string_size

        // Record data (1 record, 2 fields)
        data.extend_from_slice(&42u32.to_le_bytes()); // Field 0
        data.extend_from_slice(&0u32.to_le_bytes());  // Field 1 (string offset)

        // String table
        data.push(0); // Empty string

        let dbc = DBCFile::from_bytes("test.dbc".to_string(), &data).unwrap();

        assert_eq!(dbc.get_record_count(), 1);
        assert_eq!(dbc.get_field_count(), 2);

        let record = dbc.get_record(0).unwrap();
        assert_eq!(record.get_uint(0), 42);
    }

    #[test]
    fn test_invalid_magic() {
        let mut data = Vec::new();
        data.extend_from_slice(b"XXXX"); // Invalid magic
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());

        let result = DBCFile::from_bytes("test.dbc".to_string(), &data);
        assert!(result.is_err());
    }
}
