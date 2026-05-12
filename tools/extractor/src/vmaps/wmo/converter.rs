//! WMO to VMAP Conversion
//!
//! Converts WMO geometry to VMAP binary format for server use.
//! Output matches MaNGOS wmo.cpp ConvertToVMAPGroupWmo() and
//! vmapexport.cpp ExtractSingleWmo() formats.

use anyhow::{Context, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use glam::Vec3;
use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::path::Path;

use crate::vmaps::types::{BoundingBox, VMAP_MAGIC};
use crate::vmaps::wmo::{WMOGroup, WMORoot};

impl WMORoot {
    /// Write WMO root header to an output writer (MaNGOS-compatible)
    ///
    /// MaNGOS format (wmo.cpp ConvertToVMAPRootWmo):
    /// ```text
    /// "VMAPs05\0" (8 bytes)  - magic
    /// nVectors (u32)         - patched later with total vertex count
    /// nGroups (u32)          - patched later with real group count
    /// RootWMOID (u32)        - WMO ID from MOHD
    /// ```
    pub fn write_root_header<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(VMAP_MAGIC)?;
        writer.write_u32::<LittleEndian>(0)?; // nVectors - patched later
        writer.write_u32::<LittleEndian>(self.n_groups)?; // nGroups - patched later
        writer.write_u32::<LittleEndian>(self.wmo_id)?; // RootWMOID
        Ok(())
    }
}

impl WMOGroup {
    /// Convert WMO group to VMAP format and write to output stream
    ///
    /// Returns the number of collision triangles (nColTriangles) for this group.
    /// This is used by the parent to accumulate total vertex count.
    ///
    /// MaNGOS format (wmo.cpp ConvertToVMAPGroupWmo):
    /// ```text
    /// mogpFlags (u32)
    /// groupWMOID (u32)
    /// bbox min (3x f32)
    /// bbox max (3x f32)
    /// liquflags (u32)
    /// "GRP " chunk: batch data
    /// "INDX" chunk: triangle indices
    /// "VERT" chunk: vertices
    /// optional "LIQU" chunk: liquid data
    /// ```
    pub fn write_to_vmap<W: Write>(
        &self,
        writer: &mut W,
        _root: &WMORoot,
        precise_vector_data: bool,
    ) -> Result<u32> {
        // Write mogpFlags
        writer.write_u32::<LittleEndian>(self.mogp_flags)?;
        // Write groupWMOID
        writer.write_u32::<LittleEndian>(self.group_wmo_id)?;
        // Write bounding box
        writer.write_f32::<LittleEndian>(self.bounding_box.min.x)?;
        writer.write_f32::<LittleEndian>(self.bounding_box.min.y)?;
        writer.write_f32::<LittleEndian>(self.bounding_box.min.z)?;
        writer.write_f32::<LittleEndian>(self.bounding_box.max.x)?;
        writer.write_f32::<LittleEndian>(self.bounding_box.max.y)?;
        writer.write_f32::<LittleEndian>(self.bounding_box.max.z)?;
        // Write liquflags
        let liquflags: u32 = if self.has_liquid() { 1 } else { 0 };
        writer.write_u32::<LittleEndian>(liquflags)?;

        let n_col_triangles;

        if precise_vector_data {
            // Precise mode: write ALL triangles (matches MaNGOS precise path)
            let n_triangles = self.triangle_count();
            let n_vertices = self.vertex_count();

            // Write "GRP " chunk with MOBA batch data
            writer.write_all(b"GRP ")?;

            // Extract batch triangle counts from batch_info
            // MaNGOS: for(i=8; i<moba_size; i+=12) MobaEx[k++] = MOBA[i]
            // This extracts the triangle count from each MOBA entry
            let moba_batch = self.batch_info.len();
            let moba_size_grp = (moba_batch as u32) * 4 + 4;
            writer.write_u32::<LittleEndian>(moba_size_grp)?;
            writer.write_u32::<LittleEndian>(moba_batch as u32)?;
            for batch in &self.batch_info {
                // Write batch triangle count as u32
                writer.write_u32::<LittleEndian>(batch.count as u32)?;
            }

            // Write "INDX" chunk
            let n_indexes = (n_triangles * 3) as u32;
            writer.write_all(b"INDX")?;
            let wsize = std::mem::size_of::<u32>() as u32
                + std::mem::size_of::<u16>() as u32 * n_indexes;
            writer.write_u32::<LittleEndian>(wsize)?;
            writer.write_u32::<LittleEndian>(n_indexes)?;
            for &idx in &self.indices {
                writer.write_u16::<LittleEndian>(idx)?;
            }

            // Write "VERT" chunk
            writer.write_all(b"VERT")?;
            let vert_wsize = std::mem::size_of::<u32>() as u32
                + std::mem::size_of::<f32>() as u32 * 3 * n_vertices as u32;
            writer.write_u32::<LittleEndian>(vert_wsize)?;
            writer.write_u32::<LittleEndian>(n_vertices as u32)?;
            for &vertex in &self.vertices {
                writer.write_f32::<LittleEndian>(vertex.x)?;
                writer.write_f32::<LittleEndian>(vertex.y)?;
                writer.write_f32::<LittleEndian>(vertex.z)?;
            }

            n_col_triangles = n_triangles as u32;
        } else {
            // Non-precise mode: filter triangles by MOPY collision flags
            // MaNGOS checks: isRenderFace = (MOPY[2*i] & RENDER) && !(MOPY[2*i] & DETAIL)
            //                isCollision = MOPY[2*i] & COLLISION || isRenderFace
            const WMO_MATERIAL_RENDER: u16 = 0x04;
            const WMO_MATERIAL_COLLISION: u16 = 0x08;
            const WMO_MATERIAL_DETAIL: u16 = 0x02;

            let n_triangles = self.triangle_count();
            let mut col_indices: Vec<u16> = Vec::new();
            let mut used_vertices = vec![false; self.vertices.len()];

            for i in 0..n_triangles {
                let flags = if i < self.materials.len() {
                    self.materials[i]
                } else {
                    0
                };
                let is_render_face =
                    (flags & WMO_MATERIAL_RENDER) != 0 && (flags & WMO_MATERIAL_DETAIL) == 0;
                let is_collision = (flags & WMO_MATERIAL_COLLISION) != 0 || is_render_face;
                if !is_collision {
                    continue;
                }

                for j in 0..3 {
                    let idx = self.indices[i * 3 + j];
                    used_vertices[idx as usize] = true;
                    col_indices.push(idx);
                }
            }

            // Remap vertex indices
            let mut index_remap = vec![-1i32; self.vertices.len()];
            let mut n_col_vertices = 0u32;
            for (i, &used) in used_vertices.iter().enumerate() {
                if used {
                    index_remap[i] = n_col_vertices as i32;
                    n_col_vertices += 1;
                }
            }
            for idx in &mut col_indices {
                *idx = index_remap[*idx as usize] as u16;
            }

            let n_col_tris = col_indices.len() / 3;

            // Write "GRP " chunk
            writer.write_all(b"GRP ")?;
            let moba_batch = self.batch_info.len();
            let moba_size_grp = (moba_batch as u32) * 4 + 4;
            writer.write_u32::<LittleEndian>(moba_size_grp)?;
            writer.write_u32::<LittleEndian>(moba_batch as u32)?;
            for batch in &self.batch_info {
                writer.write_u32::<LittleEndian>(batch.count as u32)?;
            }

            // Write "INDX" chunk
            let n_indexes = col_indices.len() as u32;
            writer.write_all(b"INDX")?;
            let wsize = std::mem::size_of::<u32>() as u32
                + std::mem::size_of::<u16>() as u32 * n_indexes;
            writer.write_u32::<LittleEndian>(wsize)?;
            writer.write_u32::<LittleEndian>(n_indexes)?;
            for &idx in &col_indices {
                writer.write_u16::<LittleEndian>(idx)?;
            }

            // Write "VERT" chunk (only used vertices)
            writer.write_all(b"VERT")?;
            let vert_wsize = std::mem::size_of::<u32>() as u32
                + std::mem::size_of::<f32>() as u32 * 3 * n_col_vertices;
            writer.write_u32::<LittleEndian>(vert_wsize)?;
            writer.write_u32::<LittleEndian>(n_col_vertices)?;
            for (i, &vertex) in self.vertices.iter().enumerate() {
                if index_remap[i] >= 0 {
                    writer.write_f32::<LittleEndian>(vertex.x)?;
                    writer.write_f32::<LittleEndian>(vertex.y)?;
                    writer.write_f32::<LittleEndian>(vertex.z)?;
                }
            }

            n_col_triangles = n_col_tris as u32;
        }

        // Write "LIQU" chunk if liquid data exists
        // Note: liquid parsing is not yet implemented in the Rust parser,
        // so this block won't trigger until MLIQ parsing is added.
        // The format matches MaNGOS wmo.cpp LIQU writing.

        Ok(n_col_triangles)
    }

    /// Legacy convert_to_vmap for backward compatibility (returns Vec<u8>)
    /// Now delegates to write_to_vmap
    pub fn convert_to_vmap(
        &self,
        root: &WMORoot,
        precise_vector_data: bool,
    ) -> Result<Vec<u8>> {
        if !self.has_collision() {
            return Ok(Vec::new());
        }

        let mut buffer = Vec::new();
        self.write_to_vmap(&mut buffer, root, precise_vector_data)?;
        Ok(buffer)
    }

    /// Optimize geometry by deduplicating vertices
    pub fn optimize_geometry(&mut self) {
        if self.vertices.is_empty() {
            return;
        }

        // Build vertex -> index map to find duplicates
        let mut vertex_map: HashMap<VertexKey, u16> = HashMap::new();
        let mut new_vertices = Vec::new();
        let mut new_normals = Vec::new();
        let mut index_remap = vec![0u16; self.vertices.len()];

        let has_normals = self.normals.len() == self.vertices.len();

        for (old_idx, &vertex) in self.vertices.iter().enumerate() {
            let normal = if has_normals {
                Some(self.normals[old_idx])
            } else {
                None
            };

            let key = VertexKey::new(vertex, normal);

            let new_idx = if let Some(&existing_idx) = vertex_map.get(&key) {
                existing_idx
            } else {
                let new_idx = new_vertices.len() as u16;
                vertex_map.insert(key, new_idx);
                new_vertices.push(vertex);
                if let Some(n) = normal {
                    new_normals.push(n);
                }
                new_idx
            };

            index_remap[old_idx] = new_idx;
        }

        for index in &mut self.indices {
            if (*index as usize) < index_remap.len() {
                *index = index_remap[*index as usize];
            }
        }

        self.vertices = new_vertices;
        if !new_normals.is_empty() {
            self.normals = new_normals;
        }

        self.bounding_box = crate::vmaps::transform::calculate_bounding_box(&self.vertices);
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
    fn new(vertex: Vec3, normal: Option<Vec3>) -> Self {
        const SCALE: f32 = 1000.0;

        let (nx, ny, nz) = if let Some(n) = normal {
            (
                (n.x * SCALE) as i32,
                (n.y * SCALE) as i32,
                (n.z * SCALE) as i32,
            )
        } else {
            (0, 0, 0)
        };

        Self {
            x: (vertex.x * SCALE) as i32,
            y: (vertex.y * SCALE) as i32,
            z: (vertex.z * SCALE) as i32,
            nx,
            ny,
            nz,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_key_equality() {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(1.0, 2.0, 3.0);
        let v3 = Vec3::new(1.01, 2.0, 3.0);

        let k1 = VertexKey::new(v1, None);
        let k2 = VertexKey::new(v2, None);
        let k3 = VertexKey::new(v3, None);

        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn test_convert_to_vmap_empty_group() {
        let root = WMORoot::new();
        let group = WMOGroup::new();

        let result = group.convert_to_vmap(&root, true).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_to_vmap_with_geometry() {
        let root = WMORoot::new();
        let mut group = WMOGroup::new();

        group.vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        group.indices = vec![0, 1, 2];
        group.bounding_box = BoundingBox::new(Vec3::ZERO, Vec3::ONE);

        let result = group.convert_to_vmap(&root, true).unwrap();
        assert!(!result.is_empty());

        // Verify "GRP " chunk is present
        assert!(result.windows(4).any(|w| w == b"GRP "));
        // Verify "INDX" chunk is present
        assert!(result.windows(4).any(|w| w == b"INDX"));
        // Verify "VERT" chunk is present
        assert!(result.windows(4).any(|w| w == b"VERT"));
    }

    #[test]
    fn test_write_to_vmap_returns_triangle_count() {
        let root = WMORoot::new();
        let mut group = WMOGroup::new();

        group.vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        group.indices = vec![0, 1, 2, 1, 3, 2];
        group.bounding_box = BoundingBox::new(Vec3::ZERO, Vec3::ONE);

        let mut buffer = Vec::new();
        let n_col_triangles = group.write_to_vmap(&mut buffer, &root, true).unwrap();
        assert_eq!(n_col_triangles, 2);
    }

    #[test]
    fn test_optimize_geometry_deduplication() {
        let mut group = WMOGroup::new();

        group.vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0), // Duplicate of vertex 0
            Vec3::new(1.0, 0.0, 0.0), // Duplicate of vertex 1
        ];
        group.indices = vec![0, 1, 2, 3, 4, 2];

        group.optimize_geometry();

        assert_eq!(group.vertices.len(), 3);
        assert_eq!(group.indices[0], group.indices[3]);
        assert_eq!(group.indices[1], group.indices[4]);
    }
}
