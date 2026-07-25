//! The gameobject model list extracted alongside the VMaps.
//!
//! `vmaps/temp_gameobject_models` maps a GameObjectDisplayInfo display id to the
//! collision model file and its object-space bounding box. Ported from
//! `LoadGameObjectModelList` (GameObjectModel.cpp).

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;
use tracing::{debug, warn};

/// File name inside the vmaps directory.
pub const GAMEOBJECT_MODELS_FILE: &str = "temp_gameobject_models";

/// One entry of the gameobject model list.
#[derive(Debug, Clone)]
pub struct GameObjectModelData {
    /// Collision model file name, e.g. `Chest02.m2` (the `.vmo` is appended when loading).
    pub name: String,
    /// Object-space bounding box, before scale/rotation/translation.
    pub bound_low: [f32; 3],
    pub bound_high: [f32; 3],
}

impl GameObjectModelData {
    /// A model with a zero-volume box carries no usable collision geometry.
    pub fn has_bounds(&self) -> bool {
        self.bound_low != [0.0; 3] || self.bound_high != [0.0; 3]
    }

    /// Whether this is an M2 (doodad) model rather than a WMO.
    pub fn is_m2(&self) -> bool {
        self.name.to_ascii_lowercase().contains(".m2")
    }
}

/// Load the display-id → model mapping.
///
/// A missing file is not an error: it just means no gameobject collision is
/// available, which is how the reference behaves too.
pub fn load_gameobject_model_list(vmaps_dir: &Path) -> Result<HashMap<u32, GameObjectModelData>> {
    let path = vmaps_dir.join(GAMEOBJECT_MODELS_FILE);

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            warn!(
                "VMap: {:?} not found; spawned gameobjects will not block line of sight",
                path
            );
            return Ok(HashMap::new());
        }
        Err(e) => return Err(e.into()),
    };

    let mut models = HashMap::new();
    let mut cur = Cursor::new(&bytes[..]);
    let total = bytes.len() as u64;

    // Each record is: displayId, name length, name, bbox low, bbox high.
    while cur.position() < total {
        let display_id = match cur.read_u32::<LittleEndian>() {
            Ok(v) => v,
            Err(_) => break,
        };
        let name_length = match cur.read_u32::<LittleEndian>() {
            Ok(v) => v as usize,
            Err(_) => break,
        };

        // Guard against a truncated or corrupt file rather than allocating wildly.
        if name_length == 0 || name_length > 500 || cur.position() + name_length as u64 + 24 > total
        {
            warn!(
                "VMap: {:?} appears truncated or corrupt, stopping load",
                path
            );
            break;
        }

        let mut name_bytes = vec![0u8; name_length];
        cur.read_exact(&mut name_bytes)?;
        let name = String::from_utf8_lossy(&name_bytes).into_owned();

        let mut bound_low = [0f32; 3];
        let mut bound_high = [0f32; 3];
        cur.read_f32_into::<LittleEndian>(&mut bound_low)?;
        cur.read_f32_into::<LittleEndian>(&mut bound_high)?;

        models.insert(
            display_id,
            GameObjectModelData {
                name,
                bound_low,
                bound_high,
            },
        );
    }

    debug!(
        "VMap: loaded {} gameobject collision models from {:?}",
        models.len(),
        path
    );

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_empty_list() {
        let models = load_gameobject_model_list(Path::new("/nonexistent/vmaps")).unwrap();
        assert!(models.is_empty());
    }

    #[test]
    fn has_bounds_rejects_zero_volume_box() {
        let zero = GameObjectModelData {
            name: "Zero.m2".to_string(),
            bound_low: [0.0; 3],
            bound_high: [0.0; 3],
        };
        assert!(!zero.has_bounds());

        let real = GameObjectModelData {
            name: "Door.wmo".to_string(),
            bound_low: [-1.0, -1.0, 0.0],
            bound_high: [1.0, 1.0, 3.0],
        };
        assert!(real.has_bounds());
    }

    #[test]
    fn is_m2_distinguishes_model_kinds() {
        let m2 = GameObjectModelData {
            name: "Chest02.m2".to_string(),
            bound_low: [-1.0; 3],
            bound_high: [1.0; 3],
        };
        let wmo = GameObjectModelData {
            name: "1000Needlesbridge.wmo".to_string(),
            bound_low: [-1.0; 3],
            bound_high: [1.0; 3],
        };
        assert!(m2.is_m2());
        assert!(!wmo.is_m2());
    }

    #[test]
    fn parses_records_in_extractor_format() {
        let mut bytes = Vec::new();
        let name = b"Chest02.m2";
        bytes.extend_from_slice(&7u32.to_le_bytes());
        bytes.extend_from_slice(&(name.len() as u32).to_le_bytes());
        bytes.extend_from_slice(name);
        for v in [-1.0f32, -2.0, 0.0, 1.0, 2.0, 3.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }

        let dir = std::env::temp_dir().join(format!("oxcore-gomodels-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(GAMEOBJECT_MODELS_FILE), &bytes).unwrap();

        let models = load_gameobject_model_list(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        let entry = models.get(&7).expect("display id 7 present");
        assert_eq!(entry.name, "Chest02.m2");
        assert_eq!(entry.bound_low, [-1.0, -2.0, 0.0]);
        assert_eq!(entry.bound_high, [1.0, 2.0, 3.0]);
        assert!(entry.has_bounds());
        assert!(entry.is_m2());
    }
}
