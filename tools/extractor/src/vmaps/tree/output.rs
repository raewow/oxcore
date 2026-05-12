//! VMap Binary Output
//!
//! Writes BVH/BIH trees to binary format for server use.
//!
//! Supports two formats:
//! - VMTREE01: Legacy BVH format
//! - VMAP_7.0: Server-compatible format (MaNGOS compatible)

use crate::vmaps::dir_bin::{DirBinEntry, MOD_HAS_BOUND};
use crate::vmaps::tree::bih::BIH;
use crate::vmaps::tree::structures::{BVHNode, BVHTree, TriangleData};
use anyhow::{Context, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Magic number for legacy VMTREE files
pub const VMTREE_MAGIC: &[u8; 8] = b"VMTREE01";

/// Magic number for server-compatible VMAP files
pub const VMAP_MAGIC: &[u8; 8] = b"VMAP_7.0";

/// VMTREE file header
#[repr(C)]
#[derive(Debug, Clone)]
pub struct VmtreeHeader {
    /// Magic: "VMTREE01"
    pub magic: [u8; 8],
    /// Number of nodes in tree
    pub node_count: u32,
    /// Number of triangles
    pub triangle_count: u32,
    /// Root node index (typically last node)
    pub root_index: u32,
}

impl VmtreeHeader {
    pub fn new(tree: &BVHTree) -> Self {
        Self {
            magic: *VMTREE_MAGIC,
            node_count: tree.node_count() as u32,
            triangle_count: tree.triangle_count() as u32,
            root_index: tree.nodes.len().saturating_sub(1) as u32,
        }
    }
}

impl BVHTree {
    /// Write BVH tree to binary file
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let file = File::create(path)
            .with_context(|| format!("Failed to create vmtree file: {}", path.display()))?;
        let mut writer = BufWriter::new(file);

        // Write header
        let header = VmtreeHeader::new(self);
        write_header(&mut writer, &header)?;

        // Write bounding box
        write_bounding_box(&mut writer, &self.bounds)?;

        // Write all triangles first
        write_triangles(&mut writer, &self.triangles)?;

        // Write all nodes
        write_nodes(&mut writer, &self.nodes)?;

        writer.flush()?;
        Ok(())
    }
}

/// Write a BVH tree to a VMTREE file (convenience function)
pub fn write_vmtree_file(path: &Path, tree: &BVHTree) -> Result<()> {
    tree.write_to_file(path)
}

/// Write header to file
fn write_header<W: Write>(writer: &mut W, header: &VmtreeHeader) -> Result<()> {
    writer.write_all(&header.magic)?;
    writer.write_u32::<LittleEndian>(header.node_count)?;
    writer.write_u32::<LittleEndian>(header.triangle_count)?;
    writer.write_u32::<LittleEndian>(header.root_index)?;
    Ok(())
}

/// Write bounding box
fn write_bounding_box<W: Write>(writer: &mut W, bbox: &crate::vmaps::types::BoundingBox) -> Result<()> {
    writer.write_f32::<LittleEndian>(bbox.min.x)?;
    writer.write_f32::<LittleEndian>(bbox.min.y)?;
    writer.write_f32::<LittleEndian>(bbox.min.z)?;
    writer.write_f32::<LittleEndian>(bbox.max.x)?;
    writer.write_f32::<LittleEndian>(bbox.max.y)?;
    writer.write_f32::<LittleEndian>(bbox.max.z)?;
    Ok(())
}

/// Write triangles array
fn write_triangles<W: Write>(writer: &mut W, triangles: &[TriangleData]) -> Result<()> {
    for tri in triangles {
        // Write 3 vertices (9 floats)
        for vertex in &tri.vertices {
            writer.write_f32::<LittleEndian>(vertex.x)?;
            writer.write_f32::<LittleEndian>(vertex.y)?;
            writer.write_f32::<LittleEndian>(vertex.z)?;
        }

        // Write normal (3 floats)
        writer.write_f32::<LittleEndian>(tri.normal.x)?;
        writer.write_f32::<LittleEndian>(tri.normal.y)?;
        writer.write_f32::<LittleEndian>(tri.normal.z)?;

        // Write material ID (2 bytes)
        writer.write_u16::<LittleEndian>(tri.material_id)?;

        // Padding to align (2 bytes)
        writer.write_u16::<LittleEndian>(0)?;
    }
    Ok(())
}

/// Write nodes array
fn write_nodes<W: Write>(writer: &mut W, nodes: &[BVHNode]) -> Result<()> {
    for node in nodes {
        match node {
            BVHNode::Branch { bbox, left, right } => {
                // Node type: 0 = branch
                writer.write_u8(0)?;

                // Padding (3 bytes)
                writer.write_u8(0)?;
                writer.write_u8(0)?;
                writer.write_u8(0)?;

                // Bounding box (24 bytes)
                write_bounding_box(writer, bbox)?;

                // Child indices (8 bytes)
                writer.write_u32::<LittleEndian>(*left as u32)?;
                writer.write_u32::<LittleEndian>(*right as u32)?;

                // Padding to 40 bytes total (4 bytes)
                writer.write_u32::<LittleEndian>(0)?;
            }
            BVHNode::Leaf { bbox, triangle_indices } => {
                // Node type: 1 = leaf
                writer.write_u8(1)?;

                // Triangle count (1 byte)
                writer.write_u8(triangle_indices.len().min(255) as u8)?;

                // Padding (2 bytes)
                writer.write_u16::<LittleEndian>(0)?;

                // Bounding box (24 bytes)
                write_bounding_box(writer, bbox)?;

                // Triangle indices (up to 8 indices, 2 bytes each = 16 bytes max)
                for (i, &tri_idx) in triangle_indices.iter().enumerate() {
                    if i >= 8 {
                        break; // Max 8 triangles per leaf
                    }
                    writer.write_u16::<LittleEndian>(tri_idx as u16)?;
                }

                // Padding for remaining indices (fill to 16 bytes)
                for _ in triangle_indices.len()..8 {
                    writer.write_u16::<LittleEndian>(0)?;
                }
            }
        }
    }
    Ok(())
}

// =============================================================================
// Server-Compatible VMAP_7.0 Format Writers
// =============================================================================

/// Model spawn entry for server-compatible format
#[derive(Debug, Clone)]
pub struct ModelSpawnEntry {
    pub flags: u32,
    pub adt_id: u16,
    pub id: u32,
    pub position: glam::Vec3,
    pub rotation: glam::Vec3,
    pub scale: f32,
    pub bounds: Option<crate::vmaps::types::BoundingBox>,
    pub name: String,
}

impl ModelSpawnEntry {
    /// Create from a DirBinEntry
    pub fn from_dir_bin(entry: &DirBinEntry) -> Self {
        // bounds is already Option<BoundingBox> from dir_bin
        let bounds = if entry.flags & MOD_HAS_BOUND != 0 {
            entry.bounds.filter(|b| b.is_valid())
        } else {
            None
        };

        Self {
            flags: entry.flags,
            adt_id: entry.adt_id,
            id: entry.unique_id,
            position: entry.position,
            rotation: entry.rotation,
            scale: entry.scale,
            bounds,
            name: entry.name.clone(),
        }
    }

    /// Write to file in server format
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Write flags (with MOD_HAS_BOUND set appropriately)
        let flags = if self.bounds.is_some() {
            self.flags | MOD_HAS_BOUND
        } else {
            self.flags & !MOD_HAS_BOUND
        };
        writer.write_u32::<LittleEndian>(flags)?;

        // Write adtId
        writer.write_u16::<LittleEndian>(self.adt_id)?;

        // Write ID
        writer.write_u32::<LittleEndian>(self.id)?;

        // Write position (3 floats)
        writer.write_f32::<LittleEndian>(self.position.x)?;
        writer.write_f32::<LittleEndian>(self.position.y)?;
        writer.write_f32::<LittleEndian>(self.position.z)?;

        // Write rotation (3 floats)
        writer.write_f32::<LittleEndian>(self.rotation.x)?;
        writer.write_f32::<LittleEndian>(self.rotation.y)?;
        writer.write_f32::<LittleEndian>(self.rotation.z)?;

        // Write scale
        writer.write_f32::<LittleEndian>(self.scale)?;

        // Write bounds if present
        if let Some(bounds) = &self.bounds {
            writer.write_f32::<LittleEndian>(bounds.min.x)?;
            writer.write_f32::<LittleEndian>(bounds.min.y)?;
            writer.write_f32::<LittleEndian>(bounds.min.z)?;
            writer.write_f32::<LittleEndian>(bounds.max.x)?;
            writer.write_f32::<LittleEndian>(bounds.max.y)?;
            writer.write_f32::<LittleEndian>(bounds.max.z)?;
        }

        // Write name (length-prefixed)
        let name_len = self.name.len() as u32;
        writer.write_u32::<LittleEndian>(name_len)?;
        writer.write_all(self.name.as_bytes())?;

        Ok(())
    }
}

/// Writer for server-compatible .vmtree files
pub struct VMapTreeWriter;

impl VMapTreeWriter {
    /// Write a .vmtree file in server-compatible VMAP_7.0 format
    ///
    /// Format:
    /// - Magic: "VMAP_7.0" (8 bytes)
    /// - isTiled: u8 (1 = tiled map, 0 = global WMO)
    /// - "NODE" (4 bytes)
    /// - BIH data (bounds + tree + objects)
    /// - "GOBJ" (4 bytes)
    /// - For non-tiled maps: ModelSpawn entries
    pub fn write(
        path: &Path,
        bih: &BIH,
        is_tiled: bool,
        global_spawns: &[(ModelSpawnEntry, u32)], // (spawn, referenced_val)
    ) -> Result<()> {
        let file = File::create(path)
            .with_context(|| format!("Failed to create vmtree file: {}", path.display()))?;
        let mut writer = BufWriter::new(file);

        // Write magic
        writer.write_all(VMAP_MAGIC)?;

        // Write isTiled flag
        writer.write_u8(if is_tiled { 1 } else { 0 })?;

        // Write NODE chunk
        writer.write_all(b"NODE")?;

        // Write BIH data
        bih.write_to_file(&mut writer)?;

        // Write GOBJ chunk
        writer.write_all(b"GOBJ")?;

        // For non-tiled maps, write global spawns
        if !is_tiled {
            for (spawn, referenced_val) in global_spawns {
                spawn.write_to(&mut writer)?;
                writer.write_u32::<LittleEndian>(*referenced_val)?;
            }
        }

        writer.flush()?;
        Ok(())
    }
}

/// Writer for server-compatible .vmtile files
pub struct VMapTileWriter;

impl VMapTileWriter {
    /// Write a .vmtile file in server-compatible VMAP_7.0 format
    ///
    /// Format:
    /// - Magic: "VMAP_7.0" (8 bytes)
    /// - numSpawns: u32
    /// - [ModelSpawn + referencedVal] * numSpawns
    pub fn write(
        path: &Path,
        spawns: &[(ModelSpawnEntry, u32)], // (spawn, referenced_val)
    ) -> Result<()> {
        let file = File::create(path)
            .with_context(|| format!("Failed to create vmtile file: {}", path.display()))?;
        let mut writer = BufWriter::new(file);

        // Write magic
        writer.write_all(VMAP_MAGIC)?;

        // Write spawn count
        let num_spawns = spawns.len() as u32;
        writer.write_u32::<LittleEndian>(num_spawns)?;

        // Write each spawn
        for (spawn, referenced_val) in spawns {
            spawn.write_to(&mut writer)?;
            writer.write_u32::<LittleEndian>(*referenced_val)?;
        }

        writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{LittleEndian, ReadBytesExt};
    use crate::vmaps::tree::BVHTree;
    use glam::Vec3;
    use std::io::Cursor;

    #[test]
    fn test_vmtree_header() {
        let triangles = vec![
            TriangleData::new(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                0,
            ),
        ];

        let tree = BVHTree::from_triangles(triangles).build();
        let header = VmtreeHeader::new(&tree);

        assert_eq!(&header.magic, VMTREE_MAGIC);
        assert_eq!(header.triangle_count, 1);
        assert_eq!(header.node_count, 1);
    }

    #[test]
    fn test_write_header() {
        let mut buffer = Vec::new();
        let header = VmtreeHeader {
            magic: *VMTREE_MAGIC,
            node_count: 10,
            triangle_count: 20,
            root_index: 9,
        };

        write_header(&mut buffer, &header).unwrap();

        // Verify size: 8 (magic) + 4 + 4 + 4 = 20 bytes
        assert_eq!(buffer.len(), 20);

        // Verify magic
        assert_eq!(&buffer[0..8], VMTREE_MAGIC);
    }

    #[test]
    fn test_write_bounding_box() {
        let mut buffer = Vec::new();
        let bbox = crate::vmaps::types::BoundingBox::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
        );

        write_bounding_box(&mut buffer, &bbox).unwrap();

        // Verify size: 6 floats * 4 bytes = 24 bytes
        assert_eq!(buffer.len(), 24);
    }

    #[test]
    fn test_write_triangle() {
        let mut buffer = Vec::new();
        let triangles = vec![
            TriangleData::new(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                42,
            ),
        ];

        write_triangles(&mut buffer, &triangles).unwrap();

        // Verify size: 3 verts * 3 floats + 1 normal * 3 floats + material (u16) + padding (u16)
        // = 9*4 + 3*4 + 2 + 2 = 36 + 12 + 4 = 52 bytes
        assert_eq!(buffer.len(), 52);

        // Read back material ID to verify
        let mut cursor = Cursor::new(&buffer);
        cursor.set_position(48); // Skip to material ID position
        let material_id = cursor.read_u16::<LittleEndian>().unwrap();
        assert_eq!(material_id, 42);
    }

    #[test]
    fn test_write_branch_node() {
        let mut buffer = Vec::new();
        let bbox = crate::vmaps::types::BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        let nodes = vec![
            BVHNode::Branch {
                bbox,
                left: 1,
                right: 2,
            },
        ];

        write_nodes(&mut buffer, &nodes).unwrap();

        // Branch node: 1 (type) + 3 (padding) + 24 (bbox) + 8 (children) + 4 (padding) = 40 bytes
        assert_eq!(buffer.len(), 40);

        // Verify type
        assert_eq!(buffer[0], 0);
    }

    #[test]
    fn test_write_leaf_node() {
        let mut buffer = Vec::new();
        let bbox = crate::vmaps::types::BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        let nodes = vec![
            BVHNode::Leaf {
                bbox,
                triangle_indices: vec![0, 1, 2],
            },
        ];

        write_nodes(&mut buffer, &nodes).unwrap();

        // Leaf node: 1 (type) + 1 (count) + 2 (padding) + 24 (bbox) + 16 (indices) = 44 bytes
        assert_eq!(buffer.len(), 44);

        // Verify type and count
        assert_eq!(buffer[0], 1);
        assert_eq!(buffer[1], 3);
    }

    #[test]
    fn test_write_tree_to_file() {
        let triangles = vec![
            TriangleData::new(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                0,
            ),
            TriangleData::new(
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(3.0, 0.0, 0.0),
                Vec3::new(2.0, 1.0, 0.0),
                0,
            ),
        ];

        let tree = BVHTree::from_triangles(triangles).build();

        // Write to temp file
        let temp_path = std::env::temp_dir().join("test.vmtree");
        tree.write_to_file(&temp_path).unwrap();

        // Verify file exists and has content
        let metadata = std::fs::metadata(&temp_path).unwrap();
        assert!(metadata.len() > 0);

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }
}
