//! WMO Root File Structures
//!
//! The root WMO file contains metadata about the building:
//! - Group information
//! - Materials
//! - Doodad (M2 model) placements
//! - Portal information

use std::collections::HashSet;

use crate::vmaps::types::{
    BoundingBox, WMODoodadData, WMODoodadDef, WMODoodadSet,
    WMOGroupInfo, WMOMaterial,
};

/// WMO Root File
///
/// Contains metadata and references to WMO groups
#[derive(Debug, Clone)]
pub struct WMORoot {
    /// Number of group files (e.g., building_000.wmo, building_001.wmo, ...)
    pub n_groups: u32,
    /// Number of portals (for indoor/outdoor determination)
    pub n_portals: u32,
    /// Number of lights
    pub n_lights: u32,
    /// Number of M2 models referenced
    pub n_models: u32,
    /// Number of doodad instances
    pub n_doodads: u32,
    /// Number of doodad sets
    pub n_sets: u32,
    /// Ambient color
    pub ambient_color: u32,
    /// WMO ID
    pub wmo_id: u32,
    /// Overall bounding box
    pub bounding_box: BoundingBox,
    /// WMO flags
    pub flags: u32,

    // Parsed chunk data
    /// Materials (from MOMT chunk)
    pub materials: Vec<WMOMaterial>,
    /// Group names (from MOGN chunk)
    pub group_names: Vec<String>,
    /// Group information (from MOGI chunk)
    pub group_info: Vec<WMOGroupInfo>,
    /// Doodad (M2) filenames (from MODN chunk)
    pub doodad_names: Vec<String>,
    /// Doodad definitions/placements (from MODD chunk)
    pub doodad_defs: Vec<WMODoodadDef>,
    /// Doodad sets (from MODS chunk)
    pub doodad_sets: Vec<WMODoodadSet>,

    /// Valid doodad name indices (for filtering)
    pub valid_doodad_names: HashSet<u32>,
    /// Processed doodad data
    pub doodad_data: WMODoodadData,
}

impl WMORoot {
    /// Create a new empty WMO root
    pub fn new() -> Self {
        Self {
            n_groups: 0,
            n_portals: 0,
            n_lights: 0,
            n_models: 0,
            n_doodads: 0,
            n_sets: 0,
            ambient_color: 0,
            wmo_id: 0,
            bounding_box: BoundingBox::default(),
            flags: 0,
            materials: Vec::new(),
            group_names: Vec::new(),
            group_info: Vec::new(),
            doodad_names: Vec::new(),
            doodad_defs: Vec::new(),
            doodad_sets: Vec::new(),
            valid_doodad_names: HashSet::new(),
            doodad_data: WMODoodadData::default(),
        }
    }

    /// Get the number of groups
    pub fn group_count(&self) -> u32 {
        self.n_groups
    }

    /// Get group name by index
    pub fn group_name(&self, index: usize) -> Option<&str> {
        self.group_names.get(index).map(|s| s.as_str())
    }

    /// Get group info by index
    pub fn group_info(&self, index: usize) -> Option<&WMOGroupInfo> {
        self.group_info.get(index)
    }

    /// Check if WMO uses doodads
    pub fn has_doodads(&self) -> bool {
        self.n_doodads > 0 && !self.doodad_names.is_empty()
    }
}

impl Default for WMORoot {
    fn default() -> Self {
        Self::new()
    }
}

/// WMO Header (MOHD chunk)
#[derive(Debug, Clone, Copy)]
pub struct MOHDHeader {
    pub n_materials: u32,
    pub n_groups: u32,
    pub n_portals: u32,
    pub n_lights: u32,
    pub n_models: u32,
    pub n_doodads: u32,
    pub n_sets: u32,
    pub ambient_color: u32,
    pub wmo_id: u32,
    pub bounding_box: BoundingBox,
    pub flags: u32,
}

/// WMO Flags
pub mod flags {
    /// WMO has vertex colors
    pub const DO_NOT_ATTENUATE_VERTICES: u32 = 0x01;
    /// Unknown flag
    pub const FLAG_02: u32 = 0x02;
    /// Use skybox from MOSB chunk
    pub const USE_SKYBOX: u32 = 0x04;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wmo_root_new() {
        let root = WMORoot::new();
        assert_eq!(root.n_groups, 0);
        assert_eq!(root.materials.len(), 0);
        assert!(!root.has_doodads());
    }

    #[test]
    fn test_wmo_root_group_count() {
        let mut root = WMORoot::new();
        root.n_groups = 5;
        assert_eq!(root.group_count(), 5);
    }

    #[test]
    fn test_wmo_root_has_doodads() {
        let mut root = WMORoot::new();
        assert!(!root.has_doodads());

        root.n_doodads = 10;
        assert!(!root.has_doodads()); // Still false - no names

        root.doodad_names.push("test.m2".to_string());
        assert!(root.has_doodads());
    }
}
