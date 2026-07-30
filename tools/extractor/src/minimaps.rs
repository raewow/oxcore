//! Web live-map tile extraction.
//!
//! The Vanilla client stores minimap artwork as 256px BLP images. The two
//! continent grids are placed into one fixed Azeroth canvas so browser markers
//! can use one coordinate system.

use anyhow::{Context, Result};
use std::path::Path;
use tracing::{info, warn};
use wow_blp::{convert::blp_to_image, parser::load_blp_from_buf};

use crate::shared::mpq::ArchiveSet;

const TILE_SIZE: u32 = 256;
const AZEROTH_WIDTH: u32 = 85;
const AZEROTH_HEIGHT: u32 = 69;

pub fn extract(input: &Path, output: &Path) -> Result<()> {
    let mut archives = load_archives(input)?;
    let tiles_root = output.join("live-map").join("tiles");
    std::fs::create_dir_all(&tiles_root)
        .with_context(|| format!("failed to create {}", tiles_root.display()))?;

    let entries = archives.list_files()?;
    let mut written = 0;
    for entry in entries {
        let Some((continent, source_x, source_y)) = parse_minimap_path(&entry) else {
            continue;
        };
        let Some((target_x, target_y)) = target_tile(continent, source_x, source_y) else {
            continue;
        };

        let bytes = archives.read_file(&entry)?;
        match load_blp_from_buf(&bytes) {
            Ok(blp) => match blp_to_image(&blp, 0) {
                Ok(image) => {
                    let destination = tiles_root.join(format!("{target_x}_{target_y}.png"));
                    image.save(&destination).with_context(|| {
                        format!("failed to save minimap tile {}", destination.display())
                    })?;
                    written += 1;
                }
                Err(error) => warn!(%entry, %error, "skipping undecodable minimap texture"),
            },
            Err(error) => warn!(%entry, %error, "skipping unreadable minimap texture"),
        }
    }

    if written == 0 {
        anyhow::bail!("no Vanilla Azeroth minimap textures found in the loaded MPQ archives");
    }
    write_metadata(&tiles_root)?;
    info!(written, "extracted live-map minimap tiles");
    Ok(())
}

fn load_archives(input: &Path) -> Result<ArchiveSet> {
    let data_dir = if input.join("Data").is_dir() {
        input.join("Data")
    } else {
        input.to_path_buf()
    };
    let mut archives = ArchiveSet::new();
    for name in [
        "art.MPQ",
        "texture.MPQ",
        "common.MPQ",
        "common-2.MPQ",
        "patch.MPQ",
        "patch-2.MPQ",
        "patch-3.MPQ",
    ] {
        let path = data_dir.join(name);
        if path.is_file() {
            archives.add_archive(&path)?;
        }
    }
    if archives.is_empty() {
        anyhow::bail!("no texture MPQ archives found in {}", data_dir.display());
    }
    Ok(archives)
}

fn parse_minimap_path(path: &str) -> Option<(&'static str, u32, u32)> {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let parts: Vec<_> = normalized.split('/').collect();
    if parts.len() != 4 || parts[0] != "world" || parts[1] != "minimaps" {
        return None;
    }
    let continent = match parts[2] {
        "azeroth" => "azeroth",
        "kalimdor" => "kalimdor",
        _ => return None,
    };
    let stem = parts[3].strip_prefix("map")?.strip_suffix(".blp")?;
    let (x, y) = stem.split_once('_')?;
    Some((continent, x.parse().ok()?, y.parse().ok()?))
}

fn target_tile(continent: &str, x: u32, y: u32) -> Option<(u32, u32)> {
    match continent {
        // The source ranges and offsets are the unmodified Vanilla portions of
        // MapCraft's Azeroth composition.
        "kalimdor" if (23..=48).contains(&x) && (9..=55).contains(&y) => {
            Some((x - 23 + 8, y - 9 + 13))
        }
        "azeroth" if (17..=45).contains(&x) && (22..=61).contains(&y) => {
            Some((x - 17 + 55, y - 22 + 19))
        }
        _ => None,
    }
}

fn write_metadata(tiles_root: &Path) -> Result<()> {
    let metadata = format!(
        "{{\"tile_size\":{TILE_SIZE},\"width\":{AZEROTH_WIDTH},\"height\":{AZEROTH_HEIGHT}}}\n"
    );
    std::fs::write(tiles_root.join("metadata.json"), metadata)
        .context("failed to write live-map metadata")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vanilla_minimap_path() {
        assert_eq!(
            parse_minimap_path("World\\Minimaps\\Azeroth\\map17_22.blp"),
            Some(("azeroth", 17, 22))
        );
    }

    #[test]
    fn places_continents_on_the_azeroth_canvas() {
        assert_eq!(target_tile("kalimdor", 23, 9), Some((8, 13)));
        assert_eq!(target_tile("azeroth", 17, 22), Some((55, 19)));
        assert_eq!(target_tile("azeroth", 16, 22), None);
    }
}
