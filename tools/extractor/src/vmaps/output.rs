//! VMAP Binary Output Writer
//!
//! Writes WMO geometry in VMAP format for server use.
//! Also writes final assembled files (.vmtree, .vmtile) compatible with server format.

use anyhow::{Context, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use glam::Vec3;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::vmaps::types::{BoundingBox, VMapGroupHeader, VMapRootHeader, WMOBatch, VMAP_MAGIC};

/// VMAP Binary Writer
pub struct VMapWriter {
    writer: BufWriter<File>,
}

impl VMapWriter {
    /// Create a new VMAP writer
    pub fn new(path: &Path) -> Result<Self> {
        let file = File::create(path)
            .with_context(|| format!("Failed to create VMAP file: {}", path.display()))?;

        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    /// Write VMAP root header (MaNGOS format)
    pub fn write_root_header(&mut self, header: &VMapRootHeader) -> Result<()> {
        // Write magic number (8 bytes)
        self.writer.write_all(&header.magic)?;

        // Write nVectors (u32)
        self.writer.write_u32::<LittleEndian>(header.n_vectors)?;

        // Write group count (u32)
        self.writer.write_u32::<LittleEndian>(header.n_groups)?;

        // Write RootWMOID (u32)
        self.writer.write_u32::<LittleEndian>(header.root_wmo_id)?;

        Ok(())
    }

    /// Write VMAP group header
    pub fn write_group_header(&mut self, header: &VMapGroupHeader) -> Result<()> {
        // Write flags
        self.writer.write_u32::<LittleEndian>(header.flags)?;

        // Write bounding box
        self.write_bounding_box(&header.bounding_box)?;

        // Write liquid flags
        self.writer.write_u32::<LittleEndian>(header.liquid_flags)?;

        // Write counts
        self.writer.write_u32::<LittleEndian>(header.n_vertices)?;
        self.writer.write_u32::<LittleEndian>(header.n_triangles)?;
        self.writer.write_u32::<LittleEndian>(header.n_batches)?;

        Ok(())
    }

    /// Write bounding box
    fn write_bounding_box(&mut self, bbox: &BoundingBox) -> Result<()> {
        // Min point
        self.writer.write_f32::<LittleEndian>(bbox.min.x)?;
        self.writer.write_f32::<LittleEndian>(bbox.min.y)?;
        self.writer.write_f32::<LittleEndian>(bbox.min.z)?;

        // Max point
        self.writer.write_f32::<LittleEndian>(bbox.max.x)?;
        self.writer.write_f32::<LittleEndian>(bbox.max.y)?;
        self.writer.write_f32::<LittleEndian>(bbox.max.z)?;

        Ok(())
    }

    /// Write vertices
    pub fn write_vertices(&mut self, vertices: &[Vec3]) -> Result<()> {
        for vertex in vertices {
            self.writer.write_f32::<LittleEndian>(vertex.x)?;
            self.writer.write_f32::<LittleEndian>(vertex.y)?;
            self.writer.write_f32::<LittleEndian>(vertex.z)?;
        }
        Ok(())
    }

    /// Write normals
    pub fn write_normals(&mut self, normals: &[Vec3]) -> Result<()> {
        self.write_vertices(normals) // Same format as vertices
    }

    /// Write triangles (as indices)
    pub fn write_triangles(&mut self, indices: &[u16]) -> Result<()> {
        for &index in indices {
            self.writer.write_u16::<LittleEndian>(index)?;
        }
        Ok(())
    }

    /// Write material IDs
    pub fn write_materials(&mut self, materials: &[u16]) -> Result<()> {
        for &material in materials {
            self.writer.write_u16::<LittleEndian>(material)?;
        }
        Ok(())
    }

    /// Write batches
    pub fn write_batches(&mut self, batches: &[WMOBatch]) -> Result<()> {
        for batch in batches {
            self.writer.write_u32::<LittleEndian>(batch.start_index)?;
            self.writer.write_u16::<LittleEndian>(batch.count)?;
            self.writer.write_u16::<LittleEndian>(batch.min_index)?;
            self.writer.write_u16::<LittleEndian>(batch.max_index)?;
            self.writer.write_u8(batch.material_id)?;
        }
        Ok(())
    }

    /// Flush and finalize the file
    pub fn finalize(mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}

/// Create a VMAP root header (MaNGOS format)
pub fn create_root_header(n_vectors: u32, n_groups: u32, root_wmo_id: u32) -> VMapRootHeader {
    VMapRootHeader {
        magic: *VMAP_MAGIC,
        n_vectors,
        n_groups,
        root_wmo_id,
    }
}

/// Create a VMAP group header
pub fn create_group_header(
    flags: u32,
    bounding_box: BoundingBox,
    liquid_flags: u32,
    n_vertices: u32,
    n_triangles: u32,
    n_batches: u32,
) -> VMapGroupHeader {
    VMapGroupHeader {
        flags,
        bounding_box,
        liquid_flags,
        n_vertices,
        n_triangles,
        n_batches,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_create_root_header() {
        let header = create_root_header(100, 5, 42);
        assert_eq!(header.magic, *VMAP_MAGIC);
        assert_eq!(header.n_vectors, 100);
        assert_eq!(header.n_groups, 5);
        assert_eq!(header.root_wmo_id, 42);
    }

    #[test]
    fn test_create_group_header() {
        let bbox = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        let header = create_group_header(0x01, bbox, 0, 100, 50, 5);

        assert_eq!(header.flags, 0x01);
        assert_eq!(header.n_vertices, 100);
        assert_eq!(header.n_triangles, 50);
        assert_eq!(header.n_batches, 5);
    }

    #[test]
    fn test_write_root_header() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let mut writer = VMapWriter::new(temp_file.path())?;

        let header = create_root_header(0, 3, 42);
        writer.write_root_header(&header)?;
        writer.finalize()?;

        // Read back and verify
        let mut file = File::open(temp_file.path())?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // Check magic (8 bytes including null terminator)
        assert_eq!(&buffer[0..8], b"VMAPs05\0");

        // Check nVectors (4 bytes at offset 8)
        let n_vectors = u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
        assert_eq!(n_vectors, 0);

        // Check nGroups (4 bytes at offset 12)
        let n_groups = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
        assert_eq!(n_groups, 3);

        // Check RootWMOID (4 bytes at offset 16)
        let root_wmo_id = u32::from_le_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);
        assert_eq!(root_wmo_id, 42);

        Ok(())
    }

    #[test]
    fn test_write_vertices() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let mut writer = VMapWriter::new(temp_file.path())?;

        let vertices = vec![
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
        ];

        writer.write_vertices(&vertices)?;
        writer.finalize()?;

        // Read back and verify size
        let mut file = File::open(temp_file.path())?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // 2 vertices * 3 floats * 4 bytes = 24 bytes
        assert_eq!(buffer.len(), 24);

        Ok(())
    }

    #[test]
    fn test_write_triangles() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let mut writer = VMapWriter::new(temp_file.path())?;

        let indices = vec![0u16, 1, 2, 3, 4, 5];
        writer.write_triangles(&indices)?;
        writer.finalize()?;

        // Read back and verify size
        let mut file = File::open(temp_file.path())?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // 6 indices * 2 bytes = 12 bytes
        assert_eq!(buffer.len(), 12);

        Ok(())
    }

    #[test]
    fn test_write_batches() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let mut writer = VMapWriter::new(temp_file.path())?;

        let batches = vec![
            WMOBatch {
                start_index: 0,
                count: 10,
                min_index: 0,
                max_index: 9,
                material_id: 1,
            },
            WMOBatch {
                start_index: 10,
                count: 20,
                min_index: 10,
                max_index: 29,
                material_id: 2,
            },
        ];

        writer.write_batches(&batches)?;
        writer.finalize()?;

        // Read back and verify size
        let mut file = File::open(temp_file.path())?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // 2 batches * 13 bytes each = 26 bytes
        // (u32 + u16 + u16 + u16 + u8 = 4 + 2 + 2 + 2 + 1 = 11 bytes... let me recount)
        // Actually: start_index(4) + count(2) + min_index(2) + max_index(2) + material_id(1) = 11 bytes
        assert_eq!(buffer.len(), 22); // 2 * 11

        Ok(())
    }
}

