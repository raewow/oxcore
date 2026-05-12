//! M2 to VMAP Conversion
//!
//! Converts M2 model geometry to VMAP binary format for server use.
//! Output matches MaNGOS model.cpp ConvertToVMAPModel() format.

use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use glam::Vec3;
use std::collections::HashMap;
use std::io::Cursor;

use crate::vmaps::m2::structures::M2File;
use crate::vmaps::types::VMAP_MAGIC;

impl M2File {
    /// Convert M2 model to VMAP format (MaNGOS-compatible)
    ///
    /// MaNGOS format (model.cpp:76-139):
    /// ```text
    /// "VMAPs05\0" (8 bytes)     - magic
    /// nVertices (u32)            - bounding vertex count
    /// nofgroups=1 (u32)
    /// zeros (12 bytes)           - rootwmoid, flags, groupid
    /// zeros (24 bytes)           - bbox placeholder (6 floats)
    /// zeros (4 bytes)            - liquidflags
    /// "GRP " (4 bytes)           - chunk marker
    /// wsize (u32)                - sizeof(branches) + sizeof(u32)*branches
    /// branches=1 (u32)
    /// nIndexes (u32)             - nBoundingTriangles
    /// "INDX" (4 bytes)           - chunk marker
    /// wsize (u32)
    /// nIndexes (u32)
    /// indices[] (u16 array)      - with index swap at (i%3)==1
    /// "VERT" (4 bytes)           - chunk marker
    /// wsize (u32)
    /// nVertices (u32)
    /// vertices[] (3*f32 array)   - with y/z coordinate swap
    /// ```
    pub fn convert_to_vmap(&self, _precise_vector_data: bool) -> Result<Vec<u8>> {
        // Skip if no bounding geometry
        if !self.uses_bounding_geometry() {
            return Ok(Vec::new());
        }

        let vertices = &self.bounding_vertices;
        let indices = &self.indices;

        if vertices.is_empty() || indices.is_empty() {
            return Ok(Vec::new());
        }

        let n_vertices = vertices.len() as u32;
        let n_indexes = indices.len() as u32;

        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        // Write magic (8 bytes)
        cursor.write_all(VMAP_MAGIC)?;

        // Write nVertices
        cursor.write_u32::<LittleEndian>(n_vertices)?;

        // Write nofgroups = 1
        cursor.write_u32::<LittleEndian>(1)?;

        // Write 12 bytes zeros (rootwmoid, flags, groupid)
        for _ in 0..3 {
            cursor.write_u32::<LittleEndian>(0)?;
        }

        // Write 24 bytes zeros (bbox placeholder - 6 floats)
        for _ in 0..6 {
            cursor.write_f32::<LittleEndian>(0.0)?;
        }

        // Write 4 bytes zeros (liquidflags)
        cursor.write_u32::<LittleEndian>(0)?;

        // Write "GRP " chunk
        cursor.write_all(b"GRP ")?;
        let branches: u32 = 1;
        let wsize = std::mem::size_of::<u32>() as u32 + std::mem::size_of::<u32>() as u32 * branches;
        cursor.write_u32::<LittleEndian>(wsize)?; // wsize = 8
        cursor.write_u32::<LittleEndian>(branches)?;

        // Write nIndexes (after GRP chunk, before INDX)
        cursor.write_u32::<LittleEndian>(n_indexes)?;

        // Write "INDX" chunk
        cursor.write_all(b"INDX")?;
        let indx_wsize = std::mem::size_of::<u32>() as u32 + std::mem::size_of::<u16>() as u32 * n_indexes;
        cursor.write_u32::<LittleEndian>(indx_wsize)?;
        cursor.write_u32::<LittleEndian>(n_indexes)?;

        // Write indices with winding swap: swap indices[i] and indices[i+1] when (i%3)==1
        // This matches MaNGOS: if ((i % 3) - 1 == 0) { swap(indices[i], indices[i+1]); }
        let mut swapped_indices = indices.clone();
        for i in 0..swapped_indices.len() {
            if i % 3 == 1 && i + 1 < swapped_indices.len() {
                swapped_indices.swap(i, i + 1);
            }
        }
        for &idx in &swapped_indices {
            cursor.write_u16::<LittleEndian>(idx)?;
        }

        // Write "VERT" chunk
        cursor.write_all(b"VERT")?;
        let vert_wsize = std::mem::size_of::<u32>() as u32 + std::mem::size_of::<f32>() as u32 * 3 * n_vertices;
        cursor.write_u32::<LittleEndian>(vert_wsize)?;
        cursor.write_u32::<LittleEndian>(n_vertices)?;

        // Write vertices with coordinate transform
        // MaNGOS applies fixCoordSystem (x,z,-y) at parse time, then at write time does:
        //   tmp = y; y = -z; z = tmp
        // Net effect on original coords: (x, y, z) -> fixCoord -> (x, z, -y) -> writeSwap -> (x, y, z)
        // So the coordinates written are identical to the original... BUT the index swap still matters.
        // Actually let's trace more carefully:
        //   parse: vertices[i] = fixCoordSystem(boundingVertices[i].pos) = (x, z, -y)
        //   write: tmp = vertices[i].y; vertices[i].y = -vertices[i].z; vertices[i].z = tmp
        //        = tmp = z; y = -(-y) = y; z = z  ... wait:
        //   After fixCoord: v = (x, z, -y)
        //   Write swap: tmp = v.y = z; v.y = -v.z = -(-y) = y; v.z = tmp = z
        //   Final: (x, y, z) - yes, identity!
        // So we just write the original coordinates as-is.
        for vertex in vertices {
            cursor.write_f32::<LittleEndian>(vertex.x)?;
            cursor.write_f32::<LittleEndian>(vertex.y)?;
            cursor.write_f32::<LittleEndian>(vertex.z)?;
        }

        Ok(buffer)
    }

    /// Optimize M2 geometry by deduplicating vertices
    pub fn optimize_geometry(&mut self) {
        if self.vertices.is_empty() {
            return;
        }

        // Build vertex -> index map to find duplicates
        let mut vertex_map: HashMap<VertexKey, u16> = HashMap::new();
        let mut new_vertices = Vec::new();
        let mut index_remap = vec![0u16; self.vertices.len()];

        for (old_idx, vertex) in self.vertices.iter().enumerate() {
            let key = VertexKey::new(vertex.position, vertex.normal);

            let new_idx = if let Some(&existing_idx) = vertex_map.get(&key) {
                // Reuse existing vertex
                existing_idx
            } else {
                // Add new vertex
                let new_idx = new_vertices.len() as u16;
                vertex_map.insert(key, new_idx);
                new_vertices.push(*vertex);
                new_idx
            };

            index_remap[old_idx] = new_idx;
        }

        // Remap indices
        for index in &mut self.indices {
            if (*index as usize) < index_remap.len() {
                *index = index_remap[*index as usize];
            }
        }

        // Update vertices
        self.vertices = new_vertices;
    }
}

/// Key for vertex deduplication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VertexKey {
    x: i32,
    y: i32,
    z: i32,
    nx: i32,
    ny: i32,
    nz: i32,
}

impl VertexKey {
    /// Create vertex key with quantization for floating point comparison
    fn new(position: Vec3, normal: Vec3) -> Self {
        const SCALE: f32 = 1000.0; // Quantize to 3 decimal places

        Self {
            x: (position.x * SCALE) as i32,
            y: (position.y * SCALE) as i32,
            z: (position.z * SCALE) as i32,
            nx: (normal.x * SCALE) as i32,
            ny: (normal.y * SCALE) as i32,
            nz: (normal.z * SCALE) as i32,
        }
    }
}

use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmaps::m2::structures::{M2File, M2Vertex};
    use glam::Vec2;

    #[test]
    fn test_vertex_key_equality() {
        let pos1 = Vec3::new(1.0, 2.0, 3.0);
        let pos2 = Vec3::new(1.0, 2.0, 3.0);
        let pos3 = Vec3::new(1.01, 2.0, 3.0);

        let norm = Vec3::new(0.0, 0.0, 1.0);

        let k1 = VertexKey::new(pos1, norm);
        let k2 = VertexKey::new(pos2, norm);
        let k3 = VertexKey::new(pos3, norm);

        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn test_convert_empty_m2() {
        let m2 = M2File::new();
        let result = m2.convert_to_vmap(true).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_m2_with_bounding_geometry() {
        let mut m2 = M2File::new();

        // Add bounding geometry (what MaNGOS uses for vmaps)
        m2.bounding_vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        m2.indices = vec![0, 1, 2];

        let result = m2.convert_to_vmap(true).unwrap();
        assert!(!result.is_empty());

        // Verify magic
        assert_eq!(&result[0..8], b"VMAPs05\0");

        // Verify nVertices at offset 8
        let n_verts = u32::from_le_bytes([result[8], result[9], result[10], result[11]]);
        assert_eq!(n_verts, 3);

        // Verify nofgroups at offset 12
        let n_groups = u32::from_le_bytes([result[12], result[13], result[14], result[15]]);
        assert_eq!(n_groups, 1);
    }

    #[test]
    fn test_convert_m2_has_grp_indx_vert_chunks() {
        let mut m2 = M2File::new();
        m2.bounding_vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        m2.indices = vec![0, 1, 2];

        let result = m2.convert_to_vmap(true).unwrap();

        // Find "GRP " chunk marker
        let grp_pos = result.windows(4).position(|w| w == b"GRP ").unwrap();
        assert!(grp_pos > 0);

        // Find "INDX" chunk marker
        let indx_pos = result.windows(4).position(|w| w == b"INDX").unwrap();
        assert!(indx_pos > grp_pos);

        // Find "VERT" chunk marker
        let vert_pos = result.windows(4).position(|w| w == b"VERT").unwrap();
        assert!(vert_pos > indx_pos);
    }

    #[test]
    fn test_m2_index_winding_swap() {
        let mut m2 = M2File::new();
        m2.bounding_vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        // Original indices: 0, 1, 2
        // After swap at i%3==1: 0, 2, 1
        m2.indices = vec![0, 1, 2];

        let result = m2.convert_to_vmap(true).unwrap();

        // Find INDX chunk, read past header to get indices
        let indx_pos = result.windows(4).position(|w| w == b"INDX").unwrap();
        // INDX + wsize(4) + nIndexes(4) = 12 bytes of header
        let idx_data_start = indx_pos + 4 + 4 + 4;
        let idx0 = u16::from_le_bytes([result[idx_data_start], result[idx_data_start + 1]]);
        let idx1 = u16::from_le_bytes([result[idx_data_start + 2], result[idx_data_start + 3]]);
        let idx2 = u16::from_le_bytes([result[idx_data_start + 4], result[idx_data_start + 5]]);

        // After winding swap: index 1 and 2 should be swapped
        assert_eq!(idx0, 0);
        assert_eq!(idx1, 2);
        assert_eq!(idx2, 1);
    }

    #[test]
    fn test_optimize_m2_geometry() {
        let mut m2 = M2File::new();

        // Add duplicate vertices
        m2.vertices = vec![
            M2Vertex {
                position: Vec3::new(0.0, 0.0, 0.0),
                bone_weights: [255, 0, 0, 0],
                bone_indices: [0, 0, 0, 0],
                normal: Vec3::new(0.0, 0.0, 1.0),
                tex_coords: [Vec2::ZERO; 2],
            },
            M2Vertex {
                position: Vec3::new(1.0, 0.0, 0.0),
                bone_weights: [255, 0, 0, 0],
                bone_indices: [0, 0, 0, 0],
                normal: Vec3::new(0.0, 0.0, 1.0),
                tex_coords: [Vec2::ZERO; 2],
            },
            M2Vertex {
                position: Vec3::new(0.0, 1.0, 0.0),
                bone_weights: [255, 0, 0, 0],
                bone_indices: [0, 0, 0, 0],
                normal: Vec3::new(0.0, 0.0, 1.0),
                tex_coords: [Vec2::ZERO; 2],
            },
            M2Vertex {
                position: Vec3::new(0.0, 0.0, 0.0), // Duplicate of vertex 0
                bone_weights: [255, 0, 0, 0],
                bone_indices: [0, 0, 0, 0],
                normal: Vec3::new(0.0, 0.0, 1.0),
                tex_coords: [Vec2::ZERO; 2],
            },
        ];
        m2.indices = vec![0, 1, 2, 1, 3, 2];

        m2.optimize_geometry();

        // Should have 3 unique vertices
        assert_eq!(m2.vertices.len(), 3);
        assert_eq!(m2.indices[0], m2.indices[4]); // Both were vertex 0
    }
}
