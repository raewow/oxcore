//! M2 Model File Structures
//!
//! Structures for parsing M2 (MD20) model files.
//! Reference: mangos/contrib/vmap_extractor/vmapextract/model.h

use glam::{Vec2, Vec3};
use crate::vmaps::types::BoundingBox;

/// M2 Model File
#[derive(Debug, Clone)]
pub struct M2File {
    pub header: M2Header,
    pub name: String,
    pub vertices: Vec<M2Vertex>,
    pub indices: Vec<u16>,
    pub skin_profiles: Vec<M2SkinProfile>,
    /// Bounding geometry vertices (simpler collision mesh)
    pub bounding_vertices: Vec<Vec3>,
}

impl M2File {
    /// Create a new empty M2 file
    pub fn new() -> Self {
        Self {
            header: M2Header::default(),
            name: String::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
            skin_profiles: Vec::new(),
            bounding_vertices: Vec::new(),
        }
    }

    /// Get the number of vertices
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get the number of triangles
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Check if model has geometry
    pub fn has_geometry(&self) -> bool {
        !self.vertices.is_empty() && !self.indices.is_empty()
    }
}

impl Default for M2File {
    fn default() -> Self {
        Self::new()
    }
}

impl M2File {
    /// Check if model uses bounding geometry for collision
    pub fn uses_bounding_geometry(&self) -> bool {
        !self.bounding_vertices.is_empty() && !self.indices.is_empty()
    }
}

/// M2 Main Header (MD20)
#[derive(Debug, Clone)]
pub struct M2Header {
    pub magic: [u8; 4],          // 'MD20'
    pub version: u32,
    pub name_length: u32,
    pub name_offset: u32,
    pub global_flags: u32,

    // Sequences and animations
    pub n_global_sequences: u32,
    pub ofs_global_sequences: u32,
    pub n_animations: u32,
    pub ofs_animations: u32,
    pub n_animation_lookup: u32,
    pub ofs_animation_lookup: u32,

    // Bones
    pub n_bones: u32,
    pub ofs_bones: u32,
    pub n_key_bone_lookup: u32,
    pub ofs_key_bone_lookup: u32,

    // Vertices
    pub n_vertices: u32,
    pub ofs_vertices: u32,
    pub n_views: u32,

    // Colors
    pub n_colors: u32,
    pub ofs_colors: u32,

    // Textures
    pub n_textures: u32,
    pub ofs_textures: u32,

    // Transparency
    pub n_transparency: u32,
    pub ofs_transparency: u32,

    // Texture animations
    pub n_texture_animations: u32,
    pub ofs_texture_animations: u32,
    pub n_texture_replace: u32,
    pub ofs_texture_replace: u32,

    // Render flags
    pub n_render_flags: u32,
    pub ofs_render_flags: u32,

    // Bone lookup
    pub n_bone_lookup_table: u32,
    pub ofs_bone_lookup_table: u32,

    // Texture lookup
    pub n_texture_lookup: u32,
    pub ofs_texture_lookup: u32,

    // Texture units
    pub n_texture_units: u32,
    pub ofs_texture_units: u32,
    pub n_transparency_lookup: u32,
    pub ofs_transparency_lookup: u32,
    pub n_texture_anim_lookup: u32,
    pub ofs_texture_anim_lookup: u32,

    // Bounding boxes
    pub bounding_box: BoundingBox,
    pub bounding_sphere_radius: f32,
    pub collision_box: BoundingBox,
    pub collision_sphere_radius: f32,

    // Bounding triangles
    pub n_bounding_triangles: u32,
    pub ofs_bounding_triangles: u32,
    pub n_bounding_vertices: u32,
    pub ofs_bounding_vertices: u32,
    pub n_bounding_normals: u32,
    pub ofs_bounding_normals: u32,

    // Attachments
    pub n_attachments: u32,
    pub ofs_attachments: u32,
    pub n_attachment_lookup: u32,
    pub ofs_attachment_lookup: u32,

    // Events
    pub n_events: u32,
    pub ofs_events: u32,

    // Lights
    pub n_lights: u32,
    pub ofs_lights: u32,

    // Cameras
    pub n_cameras: u32,
    pub ofs_cameras: u32,
    pub n_camera_lookup: u32,
    pub ofs_camera_lookup: u32,

    // Ribbon emitters
    pub n_ribbon_emitters: u32,
    pub ofs_ribbon_emitters: u32,

    // Particle emitters
    pub n_particle_emitters: u32,
    pub ofs_particle_emitters: u32,
}

impl M2Header {
    /// Check if model uses bounding geometry
    pub fn has_bounding_geometry(&self) -> bool {
        self.n_bounding_vertices > 0 && self.n_bounding_triangles > 0
    }
}

impl Default for M2Header {
    fn default() -> Self {
        Self {
            magic: *b"MD20",
            version: 0,
            name_length: 0,
            name_offset: 0,
            global_flags: 0,
            n_global_sequences: 0,
            ofs_global_sequences: 0,
            n_animations: 0,
            ofs_animations: 0,
            n_animation_lookup: 0,
            ofs_animation_lookup: 0,
            n_bones: 0,
            ofs_bones: 0,
            n_key_bone_lookup: 0,
            ofs_key_bone_lookup: 0,
            n_vertices: 0,
            ofs_vertices: 0,
            n_views: 0,
            n_colors: 0,
            ofs_colors: 0,
            n_textures: 0,
            ofs_textures: 0,
            n_transparency: 0,
            ofs_transparency: 0,
            n_texture_animations: 0,
            ofs_texture_animations: 0,
            n_texture_replace: 0,
            ofs_texture_replace: 0,
            n_render_flags: 0,
            ofs_render_flags: 0,
            n_bone_lookup_table: 0,
            ofs_bone_lookup_table: 0,
            n_texture_lookup: 0,
            ofs_texture_lookup: 0,
            n_texture_units: 0,
            ofs_texture_units: 0,
            n_transparency_lookup: 0,
            ofs_transparency_lookup: 0,
            n_texture_anim_lookup: 0,
            ofs_texture_anim_lookup: 0,
            bounding_box: BoundingBox::default(),
            bounding_sphere_radius: 0.0,
            collision_box: BoundingBox::default(),
            collision_sphere_radius: 0.0,
            n_bounding_triangles: 0,
            ofs_bounding_triangles: 0,
            n_bounding_vertices: 0,
            ofs_bounding_vertices: 0,
            n_bounding_normals: 0,
            ofs_bounding_normals: 0,
            n_attachments: 0,
            ofs_attachments: 0,
            n_attachment_lookup: 0,
            ofs_attachment_lookup: 0,
            n_events: 0,
            ofs_events: 0,
            n_lights: 0,
            ofs_lights: 0,
            n_cameras: 0,
            ofs_cameras: 0,
            n_camera_lookup: 0,
            ofs_camera_lookup: 0,
            n_ribbon_emitters: 0,
            ofs_ribbon_emitters: 0,
            n_particle_emitters: 0,
            ofs_particle_emitters: 0,
        }
    }
}

/// M2 Vertex
#[derive(Debug, Clone, Copy)]
pub struct M2Vertex {
    pub position: Vec3,
    pub bone_weights: [u8; 4],
    pub bone_indices: [u8; 4],
    pub normal: Vec3,
    pub tex_coords: [Vec2; 2],
}

impl Default for M2Vertex {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            bone_weights: [0; 4],
            bone_indices: [0; 4],
            normal: Vec3::ZERO,
            tex_coords: [Vec2::ZERO; 2],
        }
    }
}

/// M2 Skin Profile (LOD level)
#[derive(Debug, Clone)]
pub struct M2SkinProfile {
    pub vertices: Vec<u16>,       // Vertex indices
    pub indices: Vec<u16>,        // Triangle indices
    pub bones: Vec<[u8; 4]>,      // Bone indices for each submesh
    pub submeshes: Vec<M2Submesh>,
    pub batches: Vec<M2Batch>,
}

impl M2SkinProfile {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            bones: Vec::new(),
            submeshes: Vec::new(),
            batches: Vec::new(),
        }
    }
}

impl Default for M2SkinProfile {
    fn default() -> Self {
        Self::new()
    }
}

/// M2 Submesh
#[derive(Debug, Clone, Copy)]
pub struct M2Submesh {
    pub submesh_id: u16,
    pub level: u16,
    pub start_vertex: u16,
    pub n_vertices: u16,
    pub start_triangle: u16,
    pub n_triangles: u16,
    pub n_bones: u16,
    pub start_bones: u16,
    pub bone_influences: u16,
    pub root_bone: u16,
    pub center_position: Vec3,
    pub center_bounding_box: BoundingBox,
    pub radius: f32,
}

/// M2 Render Batch
#[derive(Debug, Clone, Copy)]
pub struct M2Batch {
    pub flags: u16,
    pub shader_id: u16,
    pub submesh_id: u16,
    pub submesh_id2: u16,
    pub color_index: u16,
    pub render_flags_index: u16,
    pub op_count: u16,
    pub material_layer: u16,
    pub material_index: u16,
    pub texture_count: u16,
    pub texture_combo_index: u16,
    pub texture_coord_combo_index: u16,
    pub texture_weight_combo_index: u16,
    pub texture_transform_combo_index: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m2_file_new() {
        let m2 = M2File::new();
        assert_eq!(m2.vertex_count(), 0);
        assert_eq!(m2.triangle_count(), 0);
        assert!(!m2.has_geometry());
    }

    #[test]
    fn test_m2_file_has_geometry() {
        let mut m2 = M2File::new();
        assert!(!m2.has_geometry());

        m2.vertices.push(M2Vertex::default());
        assert!(!m2.has_geometry()); // Need both vertices and indices

        m2.indices.push(0);
        assert!(m2.has_geometry());
    }

    #[test]
    fn test_m2_header_default() {
        let header = M2Header::default();
        assert_eq!(&header.magic, b"MD20");
        assert_eq!(header.n_vertices, 0);
        assert!(!header.has_bounding_geometry());
    }

    #[test]
    fn test_m2_header_has_bounding_geometry() {
        let mut header = M2Header::default();
        assert!(!header.has_bounding_geometry());

        header.n_bounding_vertices = 10;
        assert!(!header.has_bounding_geometry()); // Need both

        header.n_bounding_triangles = 5;
        assert!(header.has_bounding_geometry());
    }

    #[test]
    fn test_m2_vertex_default() {
        let vertex = M2Vertex::default();
        assert_eq!(vertex.position, Vec3::ZERO);
        assert_eq!(vertex.normal, Vec3::ZERO);
        assert_eq!(vertex.bone_weights, [0; 4]);
    }
}
