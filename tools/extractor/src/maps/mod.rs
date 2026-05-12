//! Map Terrain Data Extraction

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::shared::config::ExtractorConfig;
use crate::shared::dbc_parser::DBCFile;
use crate::shared::mpq::MpqArchive;

mod converter;

/// Extract map terrain data
pub fn extract(input: &Path, output: &Path, compress: bool, filter: Vec<u32>) -> Result<()> {
    info!("Extracting map data...");

    // Create configuration
    let config = if compress {
        info!("Float compression: Enabled");
        ExtractorConfig::with_compression()
    } else {
        ExtractorConfig::default()
    };

    // Load MPQ archives
    let mut archives = load_mpq_archives(input)?;

    // Step 1: Extract and read Map.dbc to get list of maps
    let map_ids = get_map_list(&mut archives, output, &filter)?;
    info!("Found {} map(s) to extract", map_ids.len());

    // Create maps output directory
    let maps_output = output.join("maps");
    std::fs::create_dir_all(&maps_output)
        .with_context(|| format!("Failed to create maps directory: {}", maps_output.display()))?;

    // Step 2: For each map, extract terrain data
    let mut extracted_tiles = 0;
    let mut total_tiles = 0;

    for map_id in map_ids {
        info!("Processing map {}...", map_id);

        match extract_map(map_id, &mut archives, &maps_output, &config) {
            Ok(count) => {
                total_tiles += count;
                extracted_tiles += count;
                if count > 0 {
                    info!("  Extracted {} tile(s)", count);
                }
            }
            Err(e) => {
                warn!("Failed to extract map {}: {}", map_id, e);
            }
        }
    }

    info!("✓ Extracted {} tile(s) from {} map(s)", extracted_tiles, total_tiles);

    Ok(())
}

/// Load MPQ archives from the input directory
fn load_mpq_archives(input: &Path) -> Result<Vec<MpqArchive>> {
    let mut archives = Vec::new();

    let mpq_files = [
        "terrain.MPQ",
        "dbc.MPQ",
        "model.MPQ",
        "patch.MPQ",
        "patch-2.MPQ",
        "patch-3.MPQ",
    ];

    let data_dir = input.join("Data");
    if !data_dir.exists() {
        debug!("Data directory not found");
        return Ok(archives);
    }

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

    info!("Loaded {} MPQ archive(s)", archives.len());
    Ok(archives)
}

/// Get list of map IDs from Map.dbc
fn get_map_list(
    archives: &mut [MpqArchive],
    output: &Path,
    filter: &[u32],
) -> Result<Vec<u32>> {
    // Try to read Map.dbc from output directory first (if already extracted)
    let map_dbc_path = output.join("dbc/Map.dbc");

    let map_dbc = if map_dbc_path.exists() {
        debug!("Reading Map.dbc from output directory");
        DBCFile::open(&map_dbc_path)?
    } else {
        // Extract Map.dbc from MPQ
        debug!("Extracting Map.dbc from MPQ");
        let dbc_data = extract_file_from_mpq(archives, "DBFilesClient\\Map.dbc")?;
        DBCFile::from_bytes("Map.dbc".to_string(), &dbc_data)?
    };

    // Collect map IDs
    let mut map_ids = HashSet::new();
    for record in map_dbc.iter() {
        let map_id = record.get_uint(0);

        // Apply filter if specified
        if !filter.is_empty() && !filter.contains(&map_id) {
            continue;
        }

        map_ids.insert(map_id);
    }

    let mut sorted_ids: Vec<u32> = map_ids.into_iter().collect();
    sorted_ids.sort();

    Ok(sorted_ids)
}

/// Extract a file from MPQ archives
fn extract_file_from_mpq(archives: &mut [MpqArchive], file_path: &str) -> Result<Vec<u8>> {
    for archive in archives {
        // Try with both slash types
        for path in &[file_path, &file_path.replace('\\', "/")] {
            if let Ok(data) = archive.archive.read_file(path) {
                return Ok(data);
            }
        }
    }

    anyhow::bail!("File not found in any MPQ: {}", file_path)
}

/// Extract a single map
fn extract_map(
    map_id: u32,
    archives: &mut [MpqArchive],
    output: &Path,
    config: &ExtractorConfig,
) -> Result<usize> {
    // Get map internal name from DBC (if available)
    let map_name = get_map_name(map_id);

    // Try to load WDT file
    let wdt_path = format!("World\\Maps\\{}\\{}.wdt", map_name, map_name);

    debug!("Looking for WDT: {}", wdt_path);

    // Check if WDT exists
    let wdt_data = match extract_file_from_mpq(archives, &wdt_path) {
        Ok(data) => data,
        Err(_) => {
            debug!("WDT not found for map {}", map_id);
            return Ok(0);
        }
    };

    // Parse WDT to get tile list
    let wdt = crate::shared::formats::wdt::WDTFile::from_bytes(map_name.clone(), &wdt_data)?;

    let tiles = wdt.get_existing_tiles();
    debug!("Map {} has {} tile(s)", map_id, tiles.len());

    if tiles.is_empty() {
        return Ok(0);
    }

    let mut extracted_count = 0;

    // Extract each tile
    for (tile_x, tile_y) in tiles {
        let adt_path = format!(
            "World\\Maps\\{}\\{}_{:02}_{:02}.adt",
            map_name, map_name, tile_x, tile_y
        );

        // Extract ADT data
        let adt_data = match extract_file_from_mpq(archives, &adt_path) {
            Ok(data) => data,
            Err(_) => {
                debug!("ADT not found: {}", adt_path);
                continue;
            }
        };

        // Output filename: {map_id:03}{tile_y:02}{tile_x:02}.map
        let output_filename = format!("{:03}{:02}{:02}.map", map_id, tile_y, tile_x);
        let output_path = output.join(output_filename);

        // Convert ADT to map file
        match converter::convert_adt(&adt_data, &output_path, tile_x as u32, tile_y as u32, config) {
            Ok(_) => {
                extracted_count += 1;
                debug!("Converted: {}_{:02}_{:02}", map_name, tile_x, tile_y);
            }
            Err(e) => {
                warn!("Failed to convert {}: {}", adt_path, e);
            }
        }
    }

    Ok(extracted_count)
}

/// Get map internal name from map ID
fn get_map_name(map_id: u32) -> String {
    // Common map names (can be extended)
    match map_id {
        0 => "Azeroth".to_string(),
        1 => "Kalimdor".to_string(),
        13 => "test".to_string(),
        25 => "ScottTest".to_string(),
        29 => "Test".to_string(),
        30 => "PVPZone01".to_string(),
        33 => "Shadowfang".to_string(),
        34 => "StormwindJail".to_string(),
        35 => "StormwindPrison".to_string(),
        36 => "DeadminesInstance".to_string(),
        37 => "PVPZone02".to_string(),
        42 => "Collin".to_string(),
        43 => "WailingCaverns".to_string(),
        44 => "Monastery".to_string(),
        47 => "RazorfenKraul".to_string(),
        48 => "Blackfathom".to_string(),
        70 => "Uldaman".to_string(),
        90 => "Gnomeregan".to_string(),
        109 => "SunkenTemple".to_string(),
        129 => "RazorfenDowns".to_string(),
        169 => "EmeraldDream".to_string(),
        189 => "MonasteryInstances".to_string(),
        209 => "TanarisInstance".to_string(),
        229 => "BlackRockSpire".to_string(),
        230 => "BlackrockDepths".to_string(),
        249 => "OnyxiaLairInstance".to_string(),
        269 => "CavernsOfTime".to_string(),
        289 => "SchoolofNecromancy".to_string(),
        309 => "Zul'gurub".to_string(),
        329 => "Stratholme".to_string(),
        349 => "Mauradon".to_string(),
        369 => "DeeprunTram".to_string(),
        389 => "OrgrimmarInstance".to_string(),
        409 => "MoltenCore".to_string(),
        429 => "DireMaul".to_string(),
        449 => "AlliancePVPBarracks".to_string(),
        450 => "HordePVPBarracks".to_string(),
        451 => "development".to_string(),
        469 => "BlackwingLair".to_string(),
        489 => "PVPZone03".to_string(),
        509 => "AhnQiraj".to_string(),
        529 => "PVPZone04".to_string(),
        530 => "Expansion01".to_string(), // Outland
        531 => "AhnQirajTemple".to_string(),
        533 => "Stratholme Raid".to_string(),
        534 => "HyjalPast".to_string(),
        540 => "HellfireMilitary".to_string(),
        542 => "HellfireDemon".to_string(),
        543 => "HellfireRampart".to_string(),
        544 => "HellfireRaid".to_string(),
        545 => "CoilfangPumping".to_string(),
        546 => "CoilfangMarsh".to_string(),
        547 => "CoilfangDraenei".to_string(),
        548 => "CoilfangRaid".to_string(),
        550 => "TempestKeepRaid".to_string(),
        552 => "TempestKeepArcane".to_string(),
        553 => "TempestKeepAtrium".to_string(),
        554 => "TempestKeepFactory".to_string(),
        555 => "AuchindounShadow".to_string(),
        556 => "AuchindounDemon".to_string(),
        557 => "AuchindounEthereal".to_string(),
        558 => "AuchindounDraenei".to_string(),
        560 => "HillsbradPast".to_string(),
        564 => "BlackTemple".to_string(),
        565 => "GruulsLair".to_string(),
        566 => "EyeOfTheStorm".to_string(),
        568 => "ZulAman".to_string(),
        580 => "SunwellPlateau".to_string(),
        585 => "Sunwell5ManFix".to_string(),
        _ => format!("Map{}", map_id),
    }
}
