//! M2 Model File Parser
//!
//! Parses M2 (MD20) model files from World of Warcraft.
//! Reference: mangos/contrib/vmap_extractor/vmapextract/model.h

use anyhow::{Result, bail};
use byteorder::{LittleEndian, ReadBytesExt};
use glam::{Vec2, Vec3};
use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::vmaps::m2::structures::{M2File, M2Header, M2Vertex};
use crate::vmaps::types::BoundingBox;

impl M2File {
    /// Parse M2 file from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);

        // Read header
        let header = Self::read_header(&mut cursor)?;

        // Validate magic
        if &header.magic != b"MD20" {
            bail!("Invalid M2 magic: expected MD20, got {:?}", header.magic);
        }

        // Debug: log header values for troubleshooting vertex parsing issues
        tracing::info!(
            "M2 header: version={}, n_vertices={}, ofs_vertices=0x{:X}, n_bounding_vertices={}, ofs_bounding_vertices=0x{:X}",
            header.version,
            header.n_vertices,
            header.ofs_vertices,
            header.n_bounding_vertices,
            header.ofs_bounding_vertices
        );

        // Read model name (if present) - with safety check
        let name = if header.name_length > 0 && header.name_offset > 0 {
            const MAX_NAME_LENGTH: u32 = 1000; // Reasonable max name length
            if header.name_length > MAX_NAME_LENGTH {
                bail!("M2 file has name too long: {} bytes (max: {}) - file may be corrupted",
                    header.name_length, MAX_NAME_LENGTH);
            }
            Self::read_string(data, header.name_offset as usize, header.name_length as usize)?
        } else {
            String::new()
        };

        // Read vertices (with safety bounds check)
        let vertices = if header.n_vertices > 0 && header.ofs_vertices > 0 {
            // Safety check: prevent huge allocations from corrupted data
            const MAX_VERTICES: u32 = 1_000_000; // 1 million vertices max (reasonable limit)
            if header.n_vertices > MAX_VERTICES {
                bail!("M2 file has too many vertices: {} (max: {}) - file may be corrupted", 
                    header.n_vertices, MAX_VERTICES);
            }
            Self::read_vertices(data, header.ofs_vertices, header.n_vertices)?
        } else {
            Vec::new()
        };

        // For VMAP extraction, only use bounding geometry (simpler collision mesh)
        // This matches the C++ behavior: models without bounding geometry are NOT extracted
        let (indices, bounding_vertices) = if header.has_bounding_geometry() {
            // Read bounding vertices and triangles (with safety bounds check)
            let bv = if header.n_bounding_vertices > 0 && header.ofs_bounding_vertices > 0 {
                const MAX_BOUNDING_VERTICES: u32 = 100_000; // Reasonable max for WoW models
                if header.n_bounding_vertices > MAX_BOUNDING_VERTICES {
                    // Corrupted file - C++ would crash or fail allocation
                    // We silently skip (return empty) to match C++ behavior of returning false
                    return Ok(Self {
                        header,
                        name,
                        vertices,
                        indices: Vec::new(),
                        skin_profiles: Vec::new(),
                        bounding_vertices: Vec::new(),
                    });
                }
                Self::read_bounding_vertices(data, header.ofs_bounding_vertices, header.n_bounding_vertices)?
            } else {
                Vec::new()
            };

            let bi = if header.n_bounding_triangles > 0 && header.ofs_bounding_triangles > 0 {
                const MAX_BOUNDING_TRIANGLES: u32 = 200_000; // Reasonable max (most models << this)
                if header.n_bounding_triangles > MAX_BOUNDING_TRIANGLES {
                    // Corrupted file - silently skip
                    return Ok(Self {
                        header,
                        name,
                        vertices,
                        indices: Vec::new(),
                        skin_profiles: Vec::new(),
                        bounding_vertices: Vec::new(),
                    });
                }
                Self::read_bounding_triangles(data, header.ofs_bounding_triangles, header.n_bounding_triangles)?
            } else {
                Vec::new()
            };

            // Only use bounding geometry if both vertices and indices are present
            (bi, bv)
        } else {
            // No bounding geometry - return empty (matches C++ behavior)
            (Vec::new(), Vec::new())
        };

        Ok(Self {
            header,
            name,
            vertices,
            indices,
            skin_profiles: Vec::new(),
            bounding_vertices,
        })
    }

    /// Read M2 header from cursor
    fn read_header(cursor: &mut Cursor<&[u8]>) -> Result<M2Header> {
        let mut magic = [0u8; 4];
        cursor.read_exact(&mut magic)?;

        let version = cursor.read_u32::<LittleEndian>()?;
        let name_length = cursor.read_u32::<LittleEndian>()?;
        let name_offset = cursor.read_u32::<LittleEndian>()?;
        let global_flags = cursor.read_u32::<LittleEndian>()?;

        // Sequences and animations
        let n_global_sequences = cursor.read_u32::<LittleEndian>()?;
        let ofs_global_sequences = cursor.read_u32::<LittleEndian>()?;
        let n_animations = cursor.read_u32::<LittleEndian>()?;
        let ofs_animations = cursor.read_u32::<LittleEndian>()?;
        let n_animation_lookup = cursor.read_u32::<LittleEndian>()?;
        let ofs_animation_lookup = cursor.read_u32::<LittleEndian>()?;

        // Vanilla M2 (version < 264) has two extra playableAnimationLookup fields here
        // (n/ofs pair) that later versions removed. We need to skip them to read subsequent fields correctly.
        if version < 264 {
            let _n_playable_animation_lookup = cursor.read_u32::<LittleEndian>()?;
            let _ofs_playable_animation_lookup = cursor.read_u32::<LittleEndian>()?;
        }

        // Bones
        let n_bones = cursor.read_u32::<LittleEndian>()?;
        let ofs_bones = cursor.read_u32::<LittleEndian>()?;
        let n_key_bone_lookup = cursor.read_u32::<LittleEndian>()?;
        let ofs_key_bone_lookup = cursor.read_u32::<LittleEndian>()?;

        // Vertices
        let n_vertices = cursor.read_u32::<LittleEndian>()?;
        let ofs_vertices = cursor.read_u32::<LittleEndian>()?;
        let n_views = cursor.read_u32::<LittleEndian>()?;
        let _ofs_views = cursor.read_u32::<LittleEndian>()?; // MaNGOS: ofsViews

        // Colors
        let n_colors = cursor.read_u32::<LittleEndian>()?;
        let ofs_colors = cursor.read_u32::<LittleEndian>()?;

        // Textures
        let n_textures = cursor.read_u32::<LittleEndian>()?;
        let ofs_textures = cursor.read_u32::<LittleEndian>()?;

        // Transparency
        let n_transparency = cursor.read_u32::<LittleEndian>()?;
        let ofs_transparency = cursor.read_u32::<LittleEndian>()?;

        // Unknown fields (MaNGOS: nI, ofsI)
        let _n_i = cursor.read_u32::<LittleEndian>()?;
        let _ofs_i = cursor.read_u32::<LittleEndian>()?;

        // Texture animations
        let n_texture_animations = cursor.read_u32::<LittleEndian>()?;
        let ofs_texture_animations = cursor.read_u32::<LittleEndian>()?;
        let n_texture_replace = cursor.read_u32::<LittleEndian>()?;
        let ofs_texture_replace = cursor.read_u32::<LittleEndian>()?;

        // Render flags
        let n_render_flags = cursor.read_u32::<LittleEndian>()?;
        let ofs_render_flags = cursor.read_u32::<LittleEndian>()?;

        // Bone lookup
        let n_bone_lookup_table = cursor.read_u32::<LittleEndian>()?;
        let ofs_bone_lookup_table = cursor.read_u32::<LittleEndian>()?;

        // Texture lookup
        let n_texture_lookup = cursor.read_u32::<LittleEndian>()?;
        let ofs_texture_lookup = cursor.read_u32::<LittleEndian>()?;

        // Texture units
        let n_texture_units = cursor.read_u32::<LittleEndian>()?;
        let ofs_texture_units = cursor.read_u32::<LittleEndian>()?;
        let n_transparency_lookup = cursor.read_u32::<LittleEndian>()?;
        let ofs_transparency_lookup = cursor.read_u32::<LittleEndian>()?;
        let n_texture_anim_lookup = cursor.read_u32::<LittleEndian>()?;
        let ofs_texture_anim_lookup = cursor.read_u32::<LittleEndian>()?;

        // Bounding boxes
        let bounding_box = Self::read_bounding_box(cursor)?;
        let bounding_sphere_radius = cursor.read_f32::<LittleEndian>()?;
        let collision_box = Self::read_bounding_box(cursor)?;
        let collision_sphere_radius = cursor.read_f32::<LittleEndian>()?;

        // Bounding triangles
        let n_bounding_triangles = cursor.read_u32::<LittleEndian>()?;
        let ofs_bounding_triangles = cursor.read_u32::<LittleEndian>()?;
        let n_bounding_vertices = cursor.read_u32::<LittleEndian>()?;
        let ofs_bounding_vertices = cursor.read_u32::<LittleEndian>()?;
        let n_bounding_normals = cursor.read_u32::<LittleEndian>()?;
        let ofs_bounding_normals = cursor.read_u32::<LittleEndian>()?;

        // Attachments
        let n_attachments = cursor.read_u32::<LittleEndian>()?;
        let ofs_attachments = cursor.read_u32::<LittleEndian>()?;
        let n_attachment_lookup = cursor.read_u32::<LittleEndian>()?;
        let ofs_attachment_lookup = cursor.read_u32::<LittleEndian>()?;

        // Events
        let n_events = cursor.read_u32::<LittleEndian>()?;
        let ofs_events = cursor.read_u32::<LittleEndian>()?;

        // Lights
        let n_lights = cursor.read_u32::<LittleEndian>()?;
        let ofs_lights = cursor.read_u32::<LittleEndian>()?;

        // Cameras
        let n_cameras = cursor.read_u32::<LittleEndian>()?;
        let ofs_cameras = cursor.read_u32::<LittleEndian>()?;
        let n_camera_lookup = cursor.read_u32::<LittleEndian>()?;
        let ofs_camera_lookup = cursor.read_u32::<LittleEndian>()?;

        // Ribbon emitters
        let n_ribbon_emitters = cursor.read_u32::<LittleEndian>()?;
        let ofs_ribbon_emitters = cursor.read_u32::<LittleEndian>()?;

        // Particle emitters
        let n_particle_emitters = cursor.read_u32::<LittleEndian>()?;
        let ofs_particle_emitters = cursor.read_u32::<LittleEndian>()?;

        Ok(M2Header {
            magic,
            version,
            name_length,
            name_offset,
            global_flags,
            n_global_sequences,
            ofs_global_sequences,
            n_animations,
            ofs_animations,
            n_animation_lookup,
            ofs_animation_lookup,
            n_bones,
            ofs_bones,
            n_key_bone_lookup,
            ofs_key_bone_lookup,
            n_vertices,
            ofs_vertices,
            n_views,
            n_colors,
            ofs_colors,
            n_textures,
            ofs_textures,
            n_transparency,
            ofs_transparency,
            n_texture_animations,
            ofs_texture_animations,
            n_texture_replace,
            ofs_texture_replace,
            n_render_flags,
            ofs_render_flags,
            n_bone_lookup_table,
            ofs_bone_lookup_table,
            n_texture_lookup,
            ofs_texture_lookup,
            n_texture_units,
            ofs_texture_units,
            n_transparency_lookup,
            ofs_transparency_lookup,
            n_texture_anim_lookup,
            ofs_texture_anim_lookup,
            bounding_box,
            bounding_sphere_radius,
            collision_box,
            collision_sphere_radius,
            n_bounding_triangles,
            ofs_bounding_triangles,
            n_bounding_vertices,
            ofs_bounding_vertices,
            n_bounding_normals,
            ofs_bounding_normals,
            n_attachments,
            ofs_attachments,
            n_attachment_lookup,
            ofs_attachment_lookup,
            n_events,
            ofs_events,
            n_lights,
            ofs_lights,
            n_cameras,
            ofs_cameras,
            n_camera_lookup,
            ofs_camera_lookup,
            n_ribbon_emitters,
            ofs_ribbon_emitters,
            n_particle_emitters,
            ofs_particle_emitters,
        })
    }

    /// Read bounding box from cursor
    fn read_bounding_box(cursor: &mut Cursor<&[u8]>) -> Result<BoundingBox> {
        let min_x = cursor.read_f32::<LittleEndian>()?;
        let min_y = cursor.read_f32::<LittleEndian>()?;
        let min_z = cursor.read_f32::<LittleEndian>()?;
        let max_x = cursor.read_f32::<LittleEndian>()?;
        let max_y = cursor.read_f32::<LittleEndian>()?;
        let max_z = cursor.read_f32::<LittleEndian>()?;

        Ok(BoundingBox::new(
            Vec3::new(min_x, min_y, min_z),
            Vec3::new(max_x, max_y, max_z),
        ))
    }

    /// Read string from file data
    fn read_string(data: &[u8], offset: usize, length: usize) -> Result<String> {
        if offset + length > data.len() {
            bail!("String offset out of bounds");
        }

        let bytes = &data[offset..offset + length];
        let string = String::from_utf8_lossy(bytes).to_string();
        Ok(string)
    }

    /// Read vertices from file data
    fn read_vertices(data: &[u8], offset: u32, count: u32) -> Result<Vec<M2Vertex>> {
        // Safety check: verify offset is within bounds
        if offset as usize >= data.len() {
            bail!("Vertex offset {} is beyond file size {}", offset, data.len());
        }
        
        // Safety check: verify we have enough data for all vertices
        // Each vertex is ~48 bytes (position 12 + bone_weights 4 + bone_indices 4 + normal 12 + tex_coords 16)
        const VERTEX_SIZE: usize = 48;
        let required_size = offset as usize + (count as usize * VERTEX_SIZE);
        if required_size > data.len() {
            bail!("Not enough data for {} vertices at offset {} (need {} bytes, have {})",
                count, offset, required_size, data.len());
        }

        let mut cursor = Cursor::new(data);
        cursor.seek(SeekFrom::Start(offset as u64))?;

        let mut vertices = Vec::with_capacity(count as usize);

        for _ in 0..count {
            // Position (12 bytes)
            let pos_x = cursor.read_f32::<LittleEndian>()?;
            let pos_y = cursor.read_f32::<LittleEndian>()?;
            let pos_z = cursor.read_f32::<LittleEndian>()?;

            // Bone weights (4 bytes)
            let mut bone_weights = [0u8; 4];
            cursor.read_exact(&mut bone_weights)?;

            // Bone indices (4 bytes)
            let mut bone_indices = [0u8; 4];
            cursor.read_exact(&mut bone_indices)?;

            // Normal (12 bytes)
            let norm_x = cursor.read_f32::<LittleEndian>()?;
            let norm_y = cursor.read_f32::<LittleEndian>()?;
            let norm_z = cursor.read_f32::<LittleEndian>()?;

            // Texture coordinates (2 sets, 8 bytes each = 16 bytes)
            let tex0_u = cursor.read_f32::<LittleEndian>()?;
            let tex0_v = cursor.read_f32::<LittleEndian>()?;
            let tex1_u = cursor.read_f32::<LittleEndian>()?;
            let tex1_v = cursor.read_f32::<LittleEndian>()?;

            vertices.push(M2Vertex {
                position: Vec3::new(pos_x, pos_y, pos_z),
                bone_weights,
                bone_indices,
                normal: Vec3::new(norm_x, norm_y, norm_z),
                tex_coords: [
                    Vec2::new(tex0_u, tex0_v),
                    Vec2::new(tex1_u, tex1_v),
                ],
            });
        }

        Ok(vertices)
    }

    /// Read bounding vertices (simplified Vec3 only)
    pub fn read_bounding_vertices(data: &[u8], offset: u32, count: u32) -> Result<Vec<Vec3>> {
        // Safety check: verify offset is within bounds
        if offset as usize >= data.len() {
            bail!("Bounding vertex offset {} is beyond file size {}", offset, data.len());
        }
        
        // Safety check: verify we have enough data (each Vec3 is 12 bytes)
        const VEC3_SIZE: usize = 12;
        let required_size = offset as usize + (count as usize * VEC3_SIZE);
        if required_size > data.len() {
            bail!("Not enough data for {} bounding vertices at offset {} (need {} bytes, have {})",
                count, offset, required_size, data.len());
        }

        let mut cursor = Cursor::new(data);
        cursor.seek(SeekFrom::Start(offset as u64))?;

        let mut vertices = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let x = cursor.read_f32::<LittleEndian>()?;
            let y = cursor.read_f32::<LittleEndian>()?;
            let z = cursor.read_f32::<LittleEndian>()?;
            vertices.push(Vec3::new(x, y, z));
        }

        Ok(vertices)
    }

    /// Read bounding triangles (u16 indices)
    pub fn read_bounding_triangles(data: &[u8], offset: u32, count: u32) -> Result<Vec<u16>> {
        // Safety check: verify offset is within bounds
        if offset as usize >= data.len() {
            bail!("Bounding triangle offset {} is beyond file size {}", offset, data.len());
        }
        
        // Safety check: verify we have enough data (each index is 2 bytes)
        const INDEX_SIZE: usize = 2;
        let required_size = offset as usize + (count as usize * INDEX_SIZE);
        if required_size > data.len() {
            bail!("Not enough data for {} bounding triangle indices at offset {} (need {} bytes, have {})",
                count, offset, required_size, data.len());
        }

        let mut cursor = Cursor::new(data);
        cursor.seek(SeekFrom::Start(offset as u64))?;

        let mut indices = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let index = cursor.read_u16::<LittleEndian>()?;
            indices.push(index);
        }

        Ok(indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_m2_header() -> Vec<u8> {
        let mut data = Vec::new();

        // Magic (4 bytes)
        data.extend_from_slice(b"MD20");

        // Version (4 bytes)
        data.extend_from_slice(&256u32.to_le_bytes());

        // Rest of header - create enough zeros for the entire header structure
        // The header has ~60 u32 fields + 2 bounding boxes + 2 floats
        // We already wrote 8 bytes, need ~392 more
        for _ in 0..98 {
            data.extend_from_slice(&0u32.to_le_bytes());
        }

        data
    }

    #[test]
    fn test_parse_m2_header() {
        let data = create_test_m2_header();
        eprintln!("Test header size: {} bytes", data.len());
        let result = M2File::from_bytes(&data);

        if let Err(ref e) = result {
            eprintln!("Parse error: {}", e);
        }

        assert!(result.is_ok());
        let m2 = result.unwrap();
        assert_eq!(&m2.header.magic, b"MD20");
        assert_eq!(m2.header.version, 256);
    }

    #[test]
    fn test_invalid_magic() {
        let mut data = create_test_m2_header();
        data[0..4].copy_from_slice(b"XXXX");

        let result = M2File::from_bytes(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_vertices() {
        let mut data = Vec::new();

        // Create a simple vertex (48 bytes)
        // Position (12 bytes)
        data.extend_from_slice(&1.0f32.to_le_bytes());
        data.extend_from_slice(&2.0f32.to_le_bytes());
        data.extend_from_slice(&3.0f32.to_le_bytes());

        // Bone weights (4 bytes)
        data.extend_from_slice(&[255, 0, 0, 0]);

        // Bone indices (4 bytes)
        data.extend_from_slice(&[0, 0, 0, 0]);

        // Normal (12 bytes)
        data.extend_from_slice(&0.0f32.to_le_bytes());
        data.extend_from_slice(&0.0f32.to_le_bytes());
        data.extend_from_slice(&1.0f32.to_le_bytes());

        // Tex coords (16 bytes)
        data.extend_from_slice(&0.5f32.to_le_bytes());
        data.extend_from_slice(&0.5f32.to_le_bytes());
        data.extend_from_slice(&0.0f32.to_le_bytes());
        data.extend_from_slice(&0.0f32.to_le_bytes());

        let vertices = M2File::read_vertices(&data, 0, 1).unwrap();

        assert_eq!(vertices.len(), 1);
        assert_eq!(vertices[0].position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(vertices[0].normal, Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(vertices[0].bone_weights[0], 255);
    }

    #[test]
    fn test_read_bounding_vertices() {
        let mut data = Vec::new();

        // Two Vec3 vertices (24 bytes)
        data.extend_from_slice(&1.0f32.to_le_bytes());
        data.extend_from_slice(&2.0f32.to_le_bytes());
        data.extend_from_slice(&3.0f32.to_le_bytes());

        data.extend_from_slice(&4.0f32.to_le_bytes());
        data.extend_from_slice(&5.0f32.to_le_bytes());
        data.extend_from_slice(&6.0f32.to_le_bytes());

        let vertices = M2File::read_bounding_vertices(&data, 0, 2).unwrap();

        assert_eq!(vertices.len(), 2);
        assert_eq!(vertices[0], Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(vertices[1], Vec3::new(4.0, 5.0, 6.0));
    }

    #[test]
    fn test_read_bounding_triangles() {
        let mut data = Vec::new();

        // Triangle indices (6 bytes)
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());

        let indices = M2File::read_bounding_triangles(&data, 0, 3).unwrap();

        assert_eq!(indices.len(), 3);
        assert_eq!(indices, vec![0, 1, 2]);
    }
}
