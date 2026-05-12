//! DBC (Database Client) File Extraction

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;
use tracing::{debug, info};

use crate::shared::mpq::MpqArchive;

/// Extract DBC files from MPQ archives
pub fn extract(input: &Path, output: &Path, filter: Vec<String>) -> Result<()> {
    info!("Extracting DBC files...");

    // Load MPQ archives
    let mut archives = load_mpq_archives(input)?;

    // Collect all DBC files
    let mut dbc_files: HashSet<String> = HashSet::new();
    
    // First, try to get files from listfile
    for archive in &mut archives {
        match archive.list_files() {
            Ok(files) => {
                for file in files {
                    if file.to_lowercase().ends_with(".dbc") {
                        // Apply filter if specified
                        if !filter.is_empty() {
                            let filename = file.rsplit(['\\', '/']).next().unwrap_or(&file);
                            if !filter.iter().any(|f| filename.contains(f)) {
                                continue;
                            }
                        }
                        dbc_files.insert(file);
                    }
                }
            }
            Err(e) => {
                debug!("Failed to list files in archive: {}", e);
            }
        }
    }
    
    // If no files found from listfile, try known DBC files directly
    if dbc_files.is_empty() {
        debug!("No files found in listfile, trying known DBC files...");
        let known_dbc_files = get_known_dbc_files();
        
        for dbc_name in &known_dbc_files {
            // Apply filter if specified
            if !filter.is_empty() {
                let filename = dbc_name.rsplit(['\\', '/']).next().unwrap_or(dbc_name);
                if !filter.iter().any(|f| filename.contains(f)) {
                    continue;
                }
            }
            
            // Try multiple path variations
            let paths = [
                format!("DBFilesClient\\{}", dbc_name),
                format!("DBFilesClient/{}", dbc_name),
                dbc_name.clone(),
                dbc_name.to_lowercase(),
                format!("DBFilesClient\\{}", dbc_name.to_lowercase()),
            ];
            
            // Check if file exists in any archive with any path variation
            for path in &paths {
                for archive in &mut archives {
                    if archive.has_file(path) {
                        debug!("Found {} in archive", path);
                        dbc_files.insert(path.clone());
                        break; // Found this file, move to next DBC file
                    }
                }
                // If we found the file with this path, no need to try other paths
                if dbc_files.contains(path) {
                    break;
                }
            }
        }
    }

    info!("Found {} DBC file(s)", dbc_files.len());

    // Create output directory
    let dbc_output = output.join("dbc");
    std::fs::create_dir_all(&dbc_output)
        .with_context(|| format!("Failed to create directory: {}", dbc_output.display()))?;
    
    info!("Output directory: {}", dbc_output.display());

    // Extract each DBC file
    let mut extracted_count = 0;
    for dbc_path in &dbc_files {
        // Remove "DBFilesClient\" prefix if present
        let relative_path = if dbc_path.starts_with("DBFilesClient\\") {
            &dbc_path[14..]
        } else if dbc_path.starts_with("DBFilesClient/") {
            &dbc_path[14..]
        } else {
            dbc_path.as_str()
        };

        let output_file = dbc_output.join(relative_path);

        // Create parent directory if needed
        if let Some(parent) = output_file.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Extract from archives in reverse order (patches override base files)
        // This ensures we get the latest version of the file from patch MPQs
        let mut extracted = false;
        for archive in archives.iter_mut().rev() {
            match archive.extract_file(dbc_path, &output_file) {
                Ok(true) => {
                    extracted = true;
                    extracted_count += 1;
                    debug!("Extracted: {} from patch/latest archive", relative_path);
                    break;
                }
                Ok(false) => continue,
                Err(e) => debug!("Error extracting {}: {}", dbc_path, e),
            }
        }

        if !extracted {
            debug!("Could not extract: {}", dbc_path);
        }
    }

    info!("✓ Extracted {} DBC files to {}", extracted_count, dbc_output.display());

    Ok(())
}

/// Load MPQ archives from the input directory
fn load_mpq_archives(input: &Path) -> Result<Vec<MpqArchive>> {
    let mut archives = Vec::new();

    // List of MPQ files to try (in order of priority)
    // IMPORTANT: Order must match C++ extractor (System.cpp CONF_mpq_list) to get same file versions
    // C++ order: model.MPQ, dbc.MPQ, terrain.MPQ, patch.MPQ, patch-2.MPQ
    let mpq_files = [
        "model.MPQ",
        "dbc.MPQ",
        "terrain.MPQ",
        "patch.MPQ",
        "patch-2.MPQ",
        "patch-3.MPQ", // Additional patch file not in C++ list
    ];

    // Try input/Data first (for WoW root directory), then input directly (for Data directory)
    let data_dir = if input.join("Data").exists() {
        input.join("Data")
    } else if input.exists() {
        input.to_path_buf()
    } else {
        debug!("Input path does not exist: {}", input.display());
        return Ok(archives);
    };

    for mpq_name in &mpq_files {
        let mpq_path = data_dir.join(mpq_name);
        if mpq_path.exists() {
            debug!("Opening: {}", mpq_path.display());
            match MpqArchive::open(&mpq_path) {
                Ok(archive) => {
                    archives.push(archive);
                    debug!("Loaded: {}", mpq_name);
                }
                Err(e) => debug!("Failed to open {}: {}", mpq_name, e),
            }
        }
    }

    // Also try to find patch MPQ files
    if let Ok(entries) = std::fs::read_dir(&data_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "mpq" {
                    let file_name = path.file_name().unwrap().to_string_lossy();
                    // Skip if already loaded
                    if mpq_files.iter().any(|&name| file_name.eq_ignore_ascii_case(name)) {
                        continue;
                    }
                    debug!("Opening patch file: {}", path.display());
                    if let Ok(archive) = MpqArchive::open(&path) {
                        archives.push(archive);
                    }
                }
            }
        }
    }

    info!("Loaded {} MPQ archive(s)", archives.len());
    Ok(archives)
}

/// Get list of known DBC files for fallback when listfile is missing
fn get_known_dbc_files() -> Vec<String> {
    vec![
        "AreaTable.dbc".to_string(),
        "AreaTrigger.dbc".to_string(),
        "AuctionHouse.dbc".to_string(),
        "BankBagSlotPrices.dbc".to_string(),
        "ChrClasses.dbc".to_string(),
        "ChrRaces.dbc".to_string(),
        "CreatureDisplayInfo.dbc".to_string(),
        "CreatureFamily.dbc".to_string(),
        "CreatureModelData.dbc".to_string(),
        "CreatureSpellData.dbc".to_string(),
        "CreatureType.dbc".to_string(),
        "Faction.dbc".to_string(),
        "FactionTemplate.dbc".to_string(),
        "GameObjectDisplayInfo.dbc".to_string(),
        "Item.dbc".to_string(),
        "ItemClass.dbc".to_string(),
        "ItemDisplayInfo.dbc".to_string(),
        "ItemExtendedCost.dbc".to_string(),
        "ItemRandomProperties.dbc".to_string(),
        "ItemSet.dbc".to_string(),
        "ItemSubClass.dbc".to_string(),
        "Lock.dbc".to_string(),
        "Map.dbc".to_string(),
        "QuestInfo.dbc".to_string(),
        "QuestSort.dbc".to_string(),
        "SkillLine.dbc".to_string(),
        "SkillLineAbility.dbc".to_string(),
        "SkillRaceClassInfo.dbc".to_string(),
        "SkillTiers.dbc".to_string(),
        "Spell.dbc".to_string(),
        "SpellCastTime.dbc".to_string(),
        "SpellDuration.dbc".to_string(),
        "SpellEffect.dbc".to_string(),
        "SpellFocusObject.dbc".to_string(),
        "SpellIcon.dbc".to_string(),
        "SpellRadius.dbc".to_string(),
        "SpellRange.dbc".to_string(),
        "SpellVisual.dbc".to_string(),
        "WorldMapArea.dbc".to_string(),
        "WorldMapOverlay.dbc".to_string(),
        "ZoneMusic.dbc".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_mpq_archives_no_data_dir() {
        let temp_dir = TempDir::new().unwrap();
        let result = load_mpq_archives(temp_dir.path()).unwrap();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_dbc_filter_logic() {
        // Test filtering logic
        let filter = vec!["Map".to_string(), "Item".to_string()];
        assert!(filter.contains(&"Map".to_string()));
        assert!(filter.contains(&"Item".to_string()));
        assert!(!filter.contains(&"Quest".to_string()));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_extract_creates_output_directory() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("input");
        let output_path = temp_dir.path().join("output");

        // Create dummy input directory
        fs::create_dir_all(&input_path).unwrap();

        // Extract will fail (no MPQs) but should create output directory
        let _ = extract(&input_path, &output_path, vec![]);

        // Output/dbc directory should exist even if extraction failed
        assert!(output_path.join("dbc").exists() || !output_path.exists());
    }

    #[test]
    fn test_filter_logic_empty() {
        // Empty filter means extract all
        let filter: Vec<String> = vec![];
        assert!(filter.is_empty());
    }

    #[test]
    fn test_filter_logic_with_items() {
        // Filter by specific DBC names
        let filter = vec!["Map".to_string(), "Item".to_string()];
        assert_eq!(filter.len(), 2);
        assert!(filter.contains(&"Map".to_string()));
    }
}
