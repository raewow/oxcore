//! VMap (Visual Map) Geometry Extraction

use anyhow::{Context, Result, bail};
use std::collections::{HashSet, HashMap};
use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{info, warn, debug};
use indicatif::{ProgressBar, ProgressStyle};
use byteorder::{LittleEndian, ReadBytesExt};

pub mod types;
pub mod transform;
pub mod wmo;
pub mod m2;
pub mod output;
pub mod placement;
pub mod tree;
pub mod assembly;
pub mod dir_bin;
pub mod vmo_converter;

use crate::shared::mpq::ArchiveSet;
use crate::shared::dbc_parser::DBCFile;

/// Extract VMap geometry data
pub fn extract(input: &Path, output: &Path, assemble_only: bool, placement_only: bool, filter: Vec<u32>) -> Result<()> {
    if assemble_only {
        info!("Assembling VMaps from existing data...");
        return assemble_vmaps(output, filter);
    }

    info!("Starting VMap extraction...");

    if !filter.is_empty() {
        info!("Filtering maps: {:?}", filter);
    }

    // Create output directory structure
    let vmaps_dir = output.join("vmaps");
    let buildings_dir = vmaps_dir.join("Buildings");
    fs::create_dir_all(&buildings_dir)
        .context("Failed to create Buildings directory")?;

    // Load MPQ archives
    info!("Loading MPQ archives from: {}", input.display());
    let archives = load_mpq_archives(input)?;

    // Get map list from DBC
    let map_list = load_map_list(&archives, &filter)?;
    info!("Found {} maps to process", map_list.len());

    if !placement_only {
        // Extract WMO and M2 models from maps
        extract_models(&archives, &buildings_dir, &map_list)?;

        // Extract M2 models from gameobjects
        extract_gameobject_models(&archives, &buildings_dir)?;
    } else {
        info!("Skipping model extraction (--placement-only mode)");
    }

    // Delete existing dir_bin if it exists (start fresh)
    let dir_bin_path = buildings_dir.join("dir_bin");
    if dir_bin_path.exists() {
        fs::remove_file(&dir_bin_path).ok();
    }

    // Create dir_bin writer for placement data
    let dir_bin_writer = dir_bin::DirBinWriter::new(&dir_bin_path)
        .context("Failed to create dir_bin writer")?;
    let unique_id_gen = dir_bin::UniqueIdGenerator::new();

    // Create M2 bounding box cache to avoid re-reading same files
    let m2_bounds_cache: Arc<Mutex<HashMap<String, Option<types::BoundingBox>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Process each map and write placement data to dir_bin
    info!("Writing placement data to dir_bin...");
    for map_info in &map_list {
        info!("Processing map: {} (ID: {})", map_info.name, map_info.id);
        process_map(&archives, &buildings_dir, map_info, &dir_bin_writer, &unique_id_gen, placement_only, &m2_bounds_cache)?;
    }

    // Log cache stats
    let cache = m2_bounds_cache.lock().unwrap();
    let cache_hits = cache.len();
    let with_bounds = cache.values().filter(|v| v.is_some()).count();
    info!("M2 cache: {} unique models, {} with bounds", cache_hits, with_bounds);

    // Flush dir_bin to disk
    info!("Flushing dir_bin to disk...");
    dir_bin_writer.flush()?;
    info!("✓ dir_bin written successfully");
    info!("✓ VMap extraction completed successfully");
    info!("Run with --assemble-only to build final VMAP files from extracted data");
    Ok(())
}

/// Information about a map
struct MapInfo {
    id: u32,
    name: String,
}

/// Load MPQ archives
fn load_mpq_archives(input: &Path) -> Result<ArchiveSet> {
    let mut archives = ArchiveSet::new();

    // Try input/Data first (for WoW root directory), then input directly (for Data directory)
    let data_dir = if input.join("Data").exists() {
        info!("Found Data directory at: {}", input.join("Data").display());
        input.join("Data")
    } else if input.exists() {
        info!("Using directory directly: {}", input.display());
        input.to_path_buf()
    } else {
        bail!("Input path does not exist: {}", input.display());
    };

    // MPQ archive names in priority order (base files first, then patches)
    // Patches are applied in reverse order during read (last wins)
    let archive_names = [
        // Base archives (Vanilla WoW)
        "base.MPQ",
        "dbc.MPQ",
        "fonts.MPQ",
        "interface.MPQ",
        "misc.MPQ",
        "model.MPQ",
        "sound.MPQ",
        "speech.MPQ",
        "terrain.MPQ",  // Contains WDT/ADT files
        "texture.MPQ",
        "wmo.MPQ",      // Contains WMO files
        // Expansion archives (TBC/WotLK)
        "common.MPQ",
        "common-2.MPQ",
        "expansion.MPQ",
        "lichking.MPQ",
        // Patches (applied last, highest priority)
        "patch.MPQ",
        "patch-2.MPQ",
        "patch-3.MPQ",
    ];

    for name in &archive_names {
        let path = data_dir.join(name);
        if path.exists() {
            match archives.add_archive(&path) {
                Ok(_) => info!("Loaded archive: {}", name),
                Err(e) => warn!("Failed to load {}: {}", name, e),
            }
        }
    }

    if archives.is_empty() {
        bail!("No MPQ archives found in: {}", data_dir.display());
    }

    Ok(archives)
}

/// Load map list from Map.dbc
fn load_map_list(archives: &ArchiveSet, filter: &[u32]) -> Result<Vec<MapInfo>> {
    info!("Loading Map.dbc...");

    let dbc_data = archives.read_file("DBFilesClient\\Map.dbc")
        .context("Failed to read Map.dbc")?;

    let dbc = DBCFile::from_bytes("Map.dbc".to_string(), &dbc_data)
        .context("Failed to parse Map.dbc")?;

    let mut maps = Vec::new();
    let filter_set: HashSet<u32> = filter.iter().copied().collect();

    for i in 0..dbc.get_record_count() {
        if let Some(record) = dbc.get_record(i) {
            let map_id = record.get_uint(0);

            // Apply filter if provided
            if !filter.is_empty() && !filter_set.contains(&map_id) {
                continue;
            }

            let name = record.get_string(1);
            if !name.is_empty() {
                maps.push(MapInfo {
                    id: map_id,
                    name: name.to_string(),
                });
            }
        }
    }

    Ok(maps)
}

/// Apply MaNGOS FixNameCase to a filename: first letter of each word is uppercase
/// (words separated by non-alpha), rest of base name lowercase, extension lowercase.
/// Matches reference/vmangos/contrib/vmap_extractor/vmapextract/adtfile.cpp:48
fn fix_name_case(name: &str) -> String {
    let bytes = name.as_bytes();
    let len = bytes.len();
    if len < 3 {
        return name.to_string();
    }
    let mut out: Vec<u8> = bytes.to_vec();
    // Base name: title case (capitalize after non-alpha, lowercase otherwise)
    let base_end = len - 3;
    for i in 0..base_end {
        let ch = out[i];
        let prev_alpha = i > 0 && (out[i - 1] as char).is_ascii_alphabetic();
        if prev_alpha && ch >= b'A' && ch <= b'Z' {
            out[i] |= 0x20;
        } else if !prev_alpha && ch >= b'a' && ch <= b'z' {
            out[i] &= !0x20;
        }
    }
    // Extension: lowercase
    for i in base_end..len {
        out[i] |= 0x20;
    }
    String::from_utf8(out).unwrap_or_else(|_| name.to_string())
}

/// Extract WMO and M2 models referenced by maps
fn extract_models(
    archives: &ArchiveSet,
    buildings_dir: &Path,
    maps: &[MapInfo],
) -> Result<()> {
    info!("Extracting models...");

    let mut wmo_files = HashSet::new();
    let mut m2_files = HashSet::new();

    // Scan WDT/ADT files to find referenced models
    info!("Scanning {} maps for model references...", maps.len());
    for map in maps {
        match scan_map_for_models(archives, &map.name) {
            Ok(models) => {
                let wmo_count = models.wmo_files.len();
                let m2_count = models.m2_files.len();
                info!("Map {}: Found {} WMO, {} M2", map.name, wmo_count, m2_count);
                wmo_files.extend(models.wmo_files);
                m2_files.extend(models.m2_files);
            }
            Err(e) => {
                warn!("Failed to scan map {}: {}", map.name, e);
            }
        }
    }

    info!("Total found: {} WMO files and {} M2 files", wmo_files.len(), m2_files.len());

    // Extract WMO files
    let pb = ProgressBar::new(wmo_files.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .unwrap());

    for wmo_path in &wmo_files {
        pb.set_message(format!("WMO: {}", wmo_path));
        if let Err(e) = extract_wmo_file(archives, buildings_dir, wmo_path) {
            warn!("Failed to extract WMO {}: {:#}", wmo_path, e);
        }
        pb.inc(1);
    }
    pb.finish_with_message("WMO extraction complete");

    // Extract M2 files
    let pb = ProgressBar::new(m2_files.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .unwrap());

    for m2_path in &m2_files {
        pb.set_message(format!("M2: {}", m2_path));
        if let Err(e) = extract_m2_file(archives, buildings_dir, m2_path) {
            debug!("Failed to extract M2 {}: {}", m2_path, e);
        }
        pb.inc(1);
    }
    pb.finish_with_message("M2 extraction complete");

    Ok(())
}

struct ModelRefs {
    wmo_files: HashSet<String>,
    m2_files: HashSet<String>,
}

/// Scan a map's WDT and ADT files for model references
fn scan_map_for_models(archives: &ArchiveSet, map_name: &str) -> Result<ModelRefs> {
    let mut refs = ModelRefs {
        wmo_files: HashSet::new(),
        m2_files: HashSet::new(),
    };

    // Load WDT file
    let wdt_path = format!("World\\Maps\\{}\\{}.wdt", map_name, map_name);
    debug!("Looking for WDT: {}", wdt_path);
    let wdt_data = match archives.read_file(&wdt_path) {
        Ok(data) => {
            debug!("Found WDT for {}, size: {} bytes", map_name, data.len());
            data
        }
        Err(e) => {
            debug!("WDT not found for {}: {}", map_name, e);
            return Ok(refs); // Map might not exist
        }
    };

    // Parse WDT using proper parser
    let wdt = match crate::shared::formats::wdt::WDTFile::from_bytes(map_name.to_string(), &wdt_data) {
        Ok(wdt) => wdt,
        Err(e) => {
            warn!("Failed to parse WDT for {}: {}", map_name, e);
            return Ok(refs);
        }
    };

    let existing_tiles = wdt.get_existing_tiles();
    let tile_count = existing_tiles.len();
    info!("Map {} has {} tiles", map_name, tile_count);

    if tile_count == 0 {
        return Ok(refs); // No tiles, return empty
    }

    // Process each tile that exists
    let total_tiles = existing_tiles.len();
    for (idx, (x, y)) in existing_tiles.iter().enumerate() {
        // Show progress every 50 tiles
        if idx % 50 == 0 {
            info!("  Processing ADT tiles {}/{} for map {}", idx, total_tiles, map_name);
        }

        let adt_path = format!("World\\Maps\\{}\\{}_{:02}_{:02}.adt", map_name, map_name, x, y);

        if let Ok(adt_data) = archives.read_file(&adt_path) {
            // Use the proper ADTFile parser instead of custom parsing
            match crate::shared::formats::adt::ADTFile::from_bytes(adt_data) {
                Ok(adt) => {
                    let wmo_count = adt.wmo_names.len();
                    let m2_count = adt.model_names.len();

                    if wmo_count > 0 || m2_count > 0 {
                        debug!("ADT {}_{:02}_{:02}: {} WMO, {} M2", map_name, x, y, wmo_count, m2_count);
                    }
                    refs.wmo_files.extend(adt.wmo_names);
                    refs.m2_files.extend(adt.model_names);
                }
                Err(e) => {
                    warn!("ADT parse error for {}_{:02}_{:02}: {}", map_name, x, y, e);
                }
            }
        }
    }

    Ok(refs)
}

/// Parse WDT file to get tile flags (which tiles exist)
fn parse_wdt_file(data: &[u8]) -> Result<[[bool; 64]; 64]> {
    let mut cursor = Cursor::new(data);
    let mut tile_flags = [[false; 64]; 64];

    // WDT file structure:
    // - MVER chunk (version)
    // - MPHD chunk (header)
    // - MAIN chunk (tile flags) - this is what we need

    loop {
        // Read chunk header
        let chunk_pos = cursor.position();
        if chunk_pos >= data.len() as u64 - 8 {
            break;
        }

        let mut chunk_id = [0u8; 4];
        if cursor.read_exact(&mut chunk_id).is_err() {
            break;
        }

        let chunk_size = match cursor.read_u32::<LittleEndian>() {
            Ok(size) => size,
            Err(_) => break,
        };

        if &chunk_id == b"MAIN" {
            // MAIN chunk contains 64x64 entries, each 8 bytes
            // First 4 bytes are flags, we just check if != 0
            for y in 0..64 {
                for x in 0..64 {
                    let flags = cursor.read_u32::<LittleEndian>()?;
                    let _async_id = cursor.read_u32::<LittleEndian>()?;
                    tile_flags[y][x] = flags != 0;
                }
            }
            break;
        } else {
            // Skip this chunk
            cursor.set_position(chunk_pos + 8 + chunk_size as u64);
        }
    }

    Ok(tile_flags)
}

/// Parse ADT file to extract model references
fn parse_adt_models(data: &[u8]) -> Result<ModelRefs> {
    let mut refs = ModelRefs {
        wmo_files: HashSet::new(),
        m2_files: HashSet::new(),
    };

    let mut cursor = Cursor::new(data);
    let mut wmo_names: Vec<String> = Vec::new();
    let mut m2_names: Vec<String> = Vec::new();
    let mut chunks_seen = Vec::new();

    // ADT file structure we care about:
    // - MWMO chunk (WMO filenames as null-terminated strings)
    // - MMDX chunk (M2 model filenames as null-terminated strings)
    // - MODF chunk (WMO placements) - just to know which WMOs are used
    // - MDDF chunk (M2 placements) - just to know which M2s are used

    loop {
        let chunk_pos = cursor.position();
        if chunk_pos >= data.len() as u64 - 8 {
            break;
        }

        let mut chunk_id = [0u8; 4];
        if cursor.read_exact(&mut chunk_id).is_err() {
            break;
        }

        let chunk_size = match cursor.read_u32::<LittleEndian>() {
            Ok(size) => size,
            Err(_) => break,
        };

        let chunk_name = String::from_utf8_lossy(&chunk_id);
        if chunks_seen.len() < 10 {
            chunks_seen.push(format!("{}", chunk_name));
        }

        match &chunk_id {
            b"MWMO" | b"OMWM" => {
                // WMO filenames - null-terminated string list
                debug!("Found {} chunk, size: {}", chunk_name, chunk_size);
                let start_pos = cursor.position();
                let end_pos = start_pos + chunk_size as u64;

                while cursor.position() < end_pos {
                    let mut name_bytes = Vec::new();
                    loop {
                        match cursor.read_u8() {
                            Ok(0) => break, // null terminator
                            Ok(b) => name_bytes.push(b),
                            Err(_) => break,
                        }
                    }

                    if !name_bytes.is_empty() {
                        if let Ok(name) = String::from_utf8(name_bytes) {
                            wmo_names.push(name);
                        }
                    }
                }
            }
            b"MMDX" | b"XDMM" => {
                // M2 model filenames - null-terminated string list
                debug!("Found {} chunk, size: {}", chunk_name, chunk_size);
                let start_pos = cursor.position();
                let end_pos = start_pos + chunk_size as u64;

                while cursor.position() < end_pos {
                    let mut name_bytes = Vec::new();
                    loop {
                        match cursor.read_u8() {
                            Ok(0) => break, // null terminator
                            Ok(b) => name_bytes.push(b),
                            Err(_) => break,
                        }
                    }

                    if !name_bytes.is_empty() {
                        if let Ok(name) = String::from_utf8(name_bytes) {
                            m2_names.push(name);
                        }
                    }
                }
            }
            _ => {
                // Skip this chunk
                cursor.set_position(chunk_pos + 8 + chunk_size as u64);
            }
        }
    }

    // Add all found model names to refs
    refs.wmo_files.extend(wmo_names.clone());
    refs.m2_files.extend(m2_names.clone());

    if chunks_seen.len() > 0 && (wmo_names.len() > 0 || m2_names.len() > 0) {
        info!("ADT chunks seen: {:?}, found {} WMO names, {} M2 names",
              &chunks_seen[..chunks_seen.len().min(5)], wmo_names.len(), m2_names.len());
    } else if chunks_seen.len() > 0 && wmo_names.is_empty() && m2_names.is_empty() {
        warn!("ADT had chunks {:?} but no models found", &chunks_seen[..chunks_seen.len().min(5)]);
    }

    Ok(refs)
}

/// Extract a single WMO file and convert to VMAP format
///
/// MaNGOS writes ALL groups into ONE file per WMO:
/// 1. Write root header: magic(8) + nVectors(u32,=0) + nGroups(u32) + RootWMOID(u32)
/// 2. For each group: write group data directly to same file
/// 3. Seek back and patch nVectors (offset 8) and real group count (offset 12)
fn extract_wmo_file(
    archives: &ArchiveSet,
    buildings_dir: &Path,
    wmo_path: &str,
) -> Result<()> {
    use std::io::{BufWriter, Seek, SeekFrom, Write};
    use byteorder::WriteBytesExt;

    // Get base filename (without extension)
    let base_name = Path::new(wmo_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Invalid WMO filename")?;

    // Check if WMO root already exists - apply FixNameCase to match dir_bin lookup
    let fixed_filename = fix_name_case(&format!("{}.wmo", base_name));
    let output_path = buildings_dir.join(&fixed_filename);
    if output_path.exists() {
        info!("Skipping WMO {} (already extracted)", wmo_path);
        return Ok(());
    }

    info!("Extracting WMO: {}", wmo_path);

    // Read WMO root file
    let wmo_data = archives.read_file(wmo_path)?;
    info!("  Read {} bytes for WMO root", wmo_data.len());

    // Parse WMO root
    let wmo_root = wmo::root::WMORoot::from_bytes(&wmo_data)
        .with_context(|| format!("Failed to parse WMO root: {}", wmo_path))?;
    info!("  Parsed WMO root: {} groups", wmo_root.n_groups);

    // Create output file
    let output_file = std::fs::File::create(&output_path)
        .with_context(|| format!("Failed to create WMO output: {}", output_path.display()))?;
    let mut writer = BufWriter::new(output_file);

    // Write root header (nVectors and nGroups will be patched later)
    wmo_root.write_root_header(&mut writer)?;

    let mut wmo_n_vertices: i32 = 0;
    let mut real_n_groups = wmo_root.n_groups;
    let mut file_ok = true;

    // Extract and convert each WMO group into the same file
    for group_idx in 0..wmo_root.n_groups {
        let group_filename = format!("{}_{:03}.wmo", base_name, group_idx);

        // Construct the full path in the MPQ
        let wmo_dir = Path::new(wmo_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let group_path = if wmo_dir.is_empty() {
            group_filename.clone()
        } else {
            format!("{}\\{}", wmo_dir, group_filename)
        };

        // Try to read the group file
        debug!("  Reading WMO group: {}", group_path);
        match archives.read_file(&group_path) {
            Ok(group_data) => {
                match wmo::group::WMOGroup::from_bytes(&group_data) {
                    Ok(group) => {
                        // Check ShouldSkip (matches MaNGOS)
                        if group.should_skip(&wmo_root) {
                            debug!("    Skipped WMO group {} (flags=0x{:X})", group_filename, group.mogp_flags);
                            real_n_groups -= 1;
                            continue;
                        }

                        // Write group data directly to the combined file
                        match group.write_to_vmap(&mut writer, &wmo_root, true) {
                            Ok(n_col_triangles) => {
                                wmo_n_vertices += n_col_triangles as i32;
                                debug!("    ✓ Wrote group {} ({} col triangles)", group_filename, n_col_triangles);
                            }
                            Err(e) => {
                                warn!("    Failed to convert WMO group {}: {}", group_filename, e);
                                file_ok = false;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("    Failed to parse WMO group {}: {}", group_filename, e);
                        file_ok = false;
                        break;
                    }
                }
            }
            Err(e) => {
                warn!("  Could not open all Group file for: {} ({})", base_name, e);
                file_ok = false;
                break;
            }
        }
    }

    // Flush before seeking
    writer.flush()?;

    // Patch nVectors at offset 8 and nGroups at offset 12
    // (matching MaNGOS: fseek(output, 8, SEEK_SET); fwrite(&Wmo_nVertices, ...))
    writer.seek(SeekFrom::Start(8))?;
    writer.write_i32::<LittleEndian>(wmo_n_vertices)?;
    writer.seek(SeekFrom::Start(12))?;
    writer.write_u32::<LittleEndian>(real_n_groups)?;
    writer.flush()?;
    drop(writer);

    // Delete the extracted file in case of error (matching MaNGOS)
    if !file_ok {
        std::fs::remove_file(&output_path).ok();
    } else {
        info!("  ✓ WMO extracted: {} groups, {} col triangles", real_n_groups, wmo_n_vertices);
    }

    Ok(())
}

/// Extract M2 models used by gameobjects from DBC
fn extract_gameobject_models(
    archives: &ArchiveSet,
    buildings_dir: &Path,
) -> Result<()> {
    info!("Extracting gameobject models...");

    // Read GameObjectDisplayInfo.dbc
    let dbc_data = match archives.read_file("DBFilesClient\\GameObjectDisplayInfo.dbc") {
        Ok(data) => data,
        Err(e) => {
            warn!("Failed to read GameObjectDisplayInfo.dbc: {}", e);
            return Ok(());
        }
    };

    let dbc = DBCFile::from_bytes("GameObjectDisplayInfo.dbc".to_string(), &dbc_data)
        .context("Failed to parse GameObjectDisplayInfo.dbc")?;

    let mut model_files = HashSet::new();

    // Extract model paths from DBC
    // Field 1 contains the model filename
    for i in 0..dbc.get_record_count() {
        if let Some(record) = dbc.get_record(i) {
            let model_name = record.get_string(1);

            if !model_name.is_empty() {
                // Convert .mdx to .m2 (old format to new)
                let m2_name = if model_name.ends_with(".mdx") {
                    model_name.replace(".mdx", ".m2")
                } else if model_name.ends_with(".m2") {
                    model_name.to_string()
                } else {
                    continue; // Skip non-model entries
                };

                model_files.insert(m2_name);
            }
        }
    }

    info!("Found {} gameobject models", model_files.len());

    // Extract each model
    let pb = ProgressBar::new(model_files.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .unwrap());

    for m2_path in &model_files {
        // Skip if already extracted
        let output_filename = Path::new(m2_path).file_name();
        if let Some(filename) = output_filename {
            let output_path = buildings_dir.join(filename);
            if output_path.exists() {
                pb.inc(1);
                continue;
            }
        }

        pb.set_message(format!("GO M2: {}", m2_path));
        if let Err(e) = extract_m2_file(archives, buildings_dir, m2_path) {
            debug!("Failed to extract gameobject M2 {}: {}", m2_path, e);
        }
        pb.inc(1);
    }
    pb.finish_with_message("Gameobject M2 extraction complete");

    Ok(())
}

/// Extract a single M2 file and convert to VMAP format
fn extract_m2_file(
    archives: &ArchiveSet,
    buildings_dir: &Path,
    m2_path: &str,
) -> Result<()> {
    // Get output filename - normalize .mdx/.mdl to .m2 and apply FixNameCase to match MaNGOS
    // (reference: model.cpp:249-253 renames extension, adtfile.cpp:48 FixNameCase)
    let raw_filename = Path::new(m2_path)
        .file_name()
        .and_then(|s| s.to_str())
        .context("Invalid M2 path")?;
    let ext_normalized = {
        let lower = raw_filename.to_lowercase();
        if lower.ends_with(".mdx") || lower.ends_with(".mdl") {
            format!("{}.m2", &raw_filename[..raw_filename.len() - 4])
        } else {
            raw_filename.to_string()
        }
    };
    let normalized_filename = fix_name_case(&ext_normalized);
    let output_path = buildings_dir.join(&normalized_filename);

    // Skip if already extracted
    if output_path.exists() {
        info!("Skipping M2 {} (already extracted)", m2_path);
        return Ok(());
    }

    info!("Extracting M2: {}", m2_path);

    // Read M2 file (handle both .m2 and .mdx extensions)
    // Archives typically have .m2 files, but ADT references may use .mdx
    let m2_data = archives.read_file(m2_path)
        .or_else(|_| {
            // Try alternate extension
            if m2_path.to_lowercase().ends_with(".mdx") {
                // Path has .mdx, try .m2
                let m2_alt = format!("{}.m2", &m2_path[..m2_path.len() - 4]);
                archives.read_file(&m2_alt)
            } else if m2_path.to_lowercase().ends_with(".m2") {
                // Path has .m2, try .mdx
                let mdx_alt = format!("{}.mdx", &m2_path[..m2_path.len() - 3]);
                archives.read_file(&mdx_alt)
            } else {
                Err(anyhow::anyhow!("Unknown M2 file extension: {}", m2_path))
            }
        })?;

    info!("  Read {} bytes", m2_data.len());

    // Quick check: skip if no vertices in header (saves parsing time)
    // For vanilla M2 (version < 264): n_vertices is at offset 0x3C = 60
    if m2_data.len() >= 68 {
        use byteorder::{LittleEndian, ReadBytesExt};
        use std::io::Cursor;
        let mut cursor = Cursor::new(&m2_data[..68]);
        cursor.set_position(0x3C); // Offset to n_vertices field in vanilla M2
        if let Ok(n_vertices) = cursor.read_u32::<LittleEndian>() {
            if n_vertices == 0 {
                info!("  Skipping: no render vertices");
                return Ok(()); // Skip models with no render vertices
            }
        }
    }

    // Parse M2 file (this will validate and check bounds)
    let m2_file = match m2::structures::M2File::from_bytes(&m2_data) {
        Ok(file) => file,
        Err(e) => {
            warn!("  Failed to parse M2: {}", e);
            return Ok(()); // Skip corrupted files
        }
    };

    info!("  Parsed: {} vertices, {} bounding vertices, {} indices",
        m2_file.vertices.len(), m2_file.bounding_vertices.len(), m2_file.indices.len());

    // Drop m2_data immediately after parsing to free memory
    drop(m2_data);

    // Convert M2 to VMAP format with high precision
    match m2_file.convert_to_vmap(true) {
        Ok(vmap_data) => {
            if !vmap_data.is_empty() {
                let data_len = vmap_data.len();
                fs::write(&output_path, vmap_data)?;
                info!("  ✓ Wrote {} bytes to {}", data_len, normalized_filename);
            } else {
                info!("  ✗ Empty vmap_data (no bounding geometry)");
            }
        }
        Err(e) => {
            warn!("  Failed to convert M2: {}", e);
        }
    }

    Ok(())
}

/// Get M2 collision box from archive (for calculating transformed bounds)
/// Returns the collision box in model-space, or None if not available
fn get_m2_collision_box(archives: &ArchiveSet, m2_path: &str) -> Option<types::BoundingBox> {
    // Try to read M2 file (handle .m2/.mdx extension swapping)
    let m2_data = archives.read_file(m2_path)
        .or_else(|_| {
            // Try alternate extension
            if m2_path.to_lowercase().ends_with(".mdx") {
                let m2_alt = format!("{}.m2", &m2_path[..m2_path.len() - 4]);
                archives.read_file(&m2_alt)
            } else if m2_path.to_lowercase().ends_with(".m2") {
                let mdx_alt = format!("{}.mdx", &m2_path[..m2_path.len() - 3]);
                archives.read_file(&mdx_alt)
            } else {
                Err(anyhow::anyhow!("Unknown M2 file extension"))
            }
        })
        .ok()?;

    // Parse M2 file to get collision box
    let m2_file = m2::structures::M2File::from_bytes(&m2_data).ok()?;

    // Try header boxes first, then fall back to computing from vertices
    // Vanilla M2 files often have invalid header bounding_box values (all zeros with negative z)
    if m2_file.header.collision_box.is_valid() {
        return Some(m2_file.header.collision_box);
    }

    if m2_file.header.bounding_box.is_valid() {
        return Some(m2_file.header.bounding_box);
    }

    // Fall back: compute bounds from render vertices
    // This is what MaNGOS does for vanilla M2 models that have no valid header bounds
    if !m2_file.vertices.is_empty() {
        let vertex_positions: Vec<glam::Vec3> = m2_file.vertices.iter()
            .map(|v| v.position)
            .collect();
        let computed_bounds = transform::calculate_bounding_box(&vertex_positions);
        if computed_bounds.is_valid() {
            tracing::debug!(
                "M2 {} - computed bounds from {} vertices: ({:.2}, {:.2}, {:.2}) to ({:.2}, {:.2}, {:.2})",
                m2_path,
                m2_file.vertices.len(),
                computed_bounds.min.x, computed_bounds.min.y, computed_bounds.min.z,
                computed_bounds.max.x, computed_bounds.max.y, computed_bounds.max.z
            );
            return Some(computed_bounds);
        }
    }

    // No valid bounds available
    tracing::warn!("M2 {} - no valid bounds (header or computed)", m2_path);
    None
}

/// M2 bounding box cache type
type M2BoundsCache = Arc<Mutex<HashMap<String, Option<types::BoundingBox>>>>;

/// Process a single map to extract tile geometry
fn process_map(
    archives: &ArchiveSet,
    buildings_dir: &Path,
    map: &MapInfo,
    dir_bin_writer: &dir_bin::DirBinWriter,
    unique_id_gen: &dir_bin::UniqueIdGenerator,
    skip_m2_bounds: bool,
    _m2_bounds_cache: &M2BoundsCache,
) -> Result<()> {
    use crate::shared::formats::wdt::WDTFile;
    use crate::shared::formats::adt::ADTFile;
    use crate::vmaps::placement::{extract_wmo_placements, extract_doodad_placements};
    use crate::vmaps::transform::decode_scale;
    use crate::vmaps::types::BoundingBox;
    use std::path::Path;

    // Load WDT file
    let wdt_path = format!("World\\Maps\\{}\\{}.wdt", map.name, map.name);
    let wdt_data = match archives.read_file(&wdt_path) {
        Ok(data) => data,
        Err(_) => {
            warn!("WDT not found for map: {}", map.name);
            return Ok(());
        }
    };

    // Parse WDT to get tile list
    let wdt = WDTFile::from_bytes(map.name.clone(), &wdt_data)
        .context("Failed to parse WDT file")?;

    let existing_tiles = wdt.get_existing_tiles();
    let has_global_wmo = !wdt.wmo_placements.is_empty();

    info!("Map {} has {} tiles, {} global WMO placements",
          map.name, existing_tiles.len(), wdt.wmo_placements.len());

    // Skip maps that have neither tiles nor global WMO
    if existing_tiles.is_empty() && !has_global_wmo {
        return Ok(());
    }

    // Process global WMO placements first (instance maps like dungeons)
    let mut total_entries = 0usize;

    if has_global_wmo {
        info!("  Processing {} global WMO placements (instance map)", wdt.wmo_placements.len());

        for modf in &wdt.wmo_placements {
            // Get WMO path from MWMO chunk
            let wmo_path = wdt.wmo_names.get(modf.id as usize)
                .ok_or_else(|| anyhow::anyhow!("Invalid WMO ID {} in MODF", modf.id))?;

            // Extract WMO file geometry (if not already extracted)
            if let Err(e) = extract_wmo_file(archives, buildings_dir, wmo_path) {
                warn!("Failed to extract global WMO {}: {}", wmo_path, e);
                continue;
            }

            // Get just the filename for dir_bin entry, apply FixNameCase (matches MaNGOS)
            let raw_wmo_name = Path::new(wmo_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(wmo_path);
            let wmo_filename = fix_name_case(raw_wmo_name);

            // Generate unique ID
            let unique_id = unique_id_gen.generate();

            // Use tile coords (65, 65) for global WMO like MaNGOS does
            const GLOBAL_WMO_TILE: u32 = 65;

            // MaNGOS hardcodes WMO scale to 1.0 (wmo.cpp:596)
            let scale = 1.0_f32;

            // Global WMO (tile 65,65): MOD_HAS_BOUND | MOD_WORLDSPAWN
            let flags = dir_bin::MOD_HAS_BOUND | dir_bin::MOD_WORLDSPAWN;

            // Create bounding box from MODF data
            let bounds = Some(BoundingBox {
                min: glam::Vec3::from_array(modf.bounds_min),
                max: glam::Vec3::from_array(modf.bounds_max),
            });

            let entry = dir_bin::DirBinEntry {
                map_id: map.id,
                tile_x: GLOBAL_WMO_TILE,
                tile_y: GLOBAL_WMO_TILE,
                flags,
                adt_id: 0,
                unique_id,
                position: glam::Vec3::from_array(modf.position),
                rotation: glam::Vec3::from_array(modf.rotation),
                scale,
                bounds,
                name: wmo_filename,
            };

            dir_bin_writer.write_entry(&entry)?;
            total_entries += 1;
        }
    }

    // Process each ADT tile and write to dir_bin
    let total_tiles = existing_tiles.len();

    for (idx, (tile_x, tile_y)) in existing_tiles.iter().copied().enumerate() {
        info!("  Processing tile {}/{}: {}_{:02}_{:02}", idx + 1, total_tiles, map.name, tile_x, tile_y);

        // Load ADT file
        let adt_path = format!("World\\Maps\\{}\\{}_{:02}_{:02}.adt", map.name, map.name, tile_x, tile_y);
        let adt_data = match archives.read_file(&adt_path) {
            Ok(data) => data,
            Err(_) => {
                debug!("ADT not found: {}", adt_path);
                continue;
            }
        };

        // Parse ADT (this will consume adt_data)
        let adt = match ADTFile::from_bytes(adt_data) {
            Ok(adt) => adt,
            Err(e) => {
                debug!("Failed to parse ADT {}: {}", adt_path, e);
                continue;
            }
        };

        // Extract placements (adt is dropped after this)
        let wmo_placements = extract_wmo_placements(&adt, map.id, tile_x as u32, tile_y as u32)?;
        let doodad_placements = extract_doodad_placements(&adt, map.id, tile_x as u32, tile_y as u32)?;

        // Drop ADT data immediately after extracting what we need
        drop(adt);

        if wmo_placements.is_empty() && doodad_placements.is_empty() {
            continue; // No models in this tile
        }

        debug!("Tile {}_{:02}_{:02}: {} WMOs, {} M2s",
               map.name, tile_x, tile_y, wmo_placements.len(), doodad_placements.len());

        // Write WMO placements to dir_bin
        for wmo_placement in &wmo_placements {
            // Generate unique ID
            let unique_id = unique_id_gen.generate();

            // Get base filename with extension for model name (matches MaNGOS plain_name + FixNameCase)
            let raw_wmo_name = Path::new(&wmo_placement.filename)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&wmo_placement.filename);
            let model_name = fix_name_case(raw_wmo_name);

            // MaNGOS hardcodes WMO scale to 1.0 (wmo.cpp:596) — WMO MODF scale field
            // was added in later WoW versions and is 0 in 1.12 data.
            let scale = 1.0_f32;
            // MaNGOS WMO entries: flags = MOD_HAS_BOUND, with bounds
            // MOD_WORLDSPAWN only added for global WMO (tile 65,65)
            let mut flags = dir_bin::MOD_HAS_BOUND;
            if tile_x as u32 == 65 && tile_y as u32 == 65 {
                flags |= dir_bin::MOD_WORLDSPAWN;
            }

            let entry = dir_bin::DirBinEntry {
                map_id: map.id,
                tile_x: tile_x as u32,
                tile_y: tile_y as u32,
                flags,
                adt_id: 0,
                unique_id,
                position: wmo_placement.placement.position,
                rotation: wmo_placement.placement.rotation,
                scale,
                bounds: Some(wmo_placement.placement.bounding_box),
                name: model_name.clone(),
            };

            if let Err(e) = dir_bin_writer.write_entry(&entry) {
                warn!("Failed to write WMO entry to dir_bin: {}", e);
            } else {
                total_entries += 1;
                if total_entries % 100 == 1 {
                    debug!("Wrote {} entries so far (last: WMO {})", total_entries, &model_name);
                }
            }
        }

        // Write M2 doodad placements to dir_bin
        for doodad_placement in &doodad_placements {
            // Generate unique ID
            let unique_id = unique_id_gen.generate();

            // Get base filename for model name, normalize extension and apply FixNameCase
            // to match on-disk filename (MaNGOS model.cpp:249-253 + adtfile.cpp:48)
            let raw_name = Path::new(&doodad_placement.filename)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&doodad_placement.filename);
            let ext_normalized = {
                let lower = raw_name.to_lowercase();
                if lower.ends_with(".mdx") || lower.ends_with(".mdl") {
                    format!("{}.m2", &raw_name[..raw_name.len() - 4])
                } else {
                    raw_name.to_string()
                }
            };
            let model_name = fix_name_case(&ext_normalized);

            let scale = decode_scale(doodad_placement.placement.scale);

            // MaNGOS M2 dir_bin entry: flags = MOD_M2, no bounds written.
            // MaNGOS (model.cpp:261-267) opens the already-extracted .m2 vmap file
            // and reads nVertices at offset 8 — if 0 or file missing, skip the doodad.
            // We replicate this by checking the on-disk file (extract_m2_file already
            // filtered out 0-render-vertex models).
            let flags_and_bounds = if skip_m2_bounds {
                // Placement-only mode: use MOD_M2 flag, no bounds
                let mut flags = dir_bin::MOD_M2;
                if tile_x as u32 == 65 && tile_y as u32 == 65 {
                    flags |= dir_bin::MOD_WORLDSPAWN;
                }
                Some((flags, None))
            } else {
                let model_path = buildings_dir.join(&model_name);
                if model_path.exists() {
                    let mut flags = dir_bin::MOD_M2;
                    if tile_x as u32 == 65 && tile_y as u32 == 65 {
                        flags |= dir_bin::MOD_WORLDSPAWN;
                    }
                    Some((flags, None))
                } else {
                    // Extracted .m2 file doesn't exist (model had nVertices == 0
                    // or failed to extract) — skip like MaNGOS does.
                    None
                }
            };

            // Skip models without collision bounds
            let Some((flags, bounds)) = flags_and_bounds else {
                continue;
            };

            let entry = dir_bin::DirBinEntry {
                map_id: map.id,
                tile_x: tile_x as u32,
                tile_y: tile_y as u32,
                flags,
                adt_id: 0,
                unique_id,
                position: doodad_placement.placement.position,
                rotation: doodad_placement.placement.rotation,
                scale,
                bounds,
                name: model_name,
            };

            if let Err(e) = dir_bin_writer.write_entry(&entry) {
                warn!("Failed to write M2 entry to dir_bin: {}", e);
            } else {
                total_entries += 1;
            }
        }

    }

    info!("Wrote {} entries to dir_bin for map {}", total_entries, map.name);
    Ok(())
}

/// Assemble VMaps from extracted data
fn assemble_vmaps(output: &Path, filter: Vec<u32>) -> Result<()> {
    info!("Starting VMap assembly...");

    let vmaps_dir = output.join("vmaps");
    let buildings_dir = vmaps_dir.join("Buildings");

    if !buildings_dir.exists() {
        bail!("Buildings directory not found. Run extraction first.");
    }

    // Read dir_bin file
    let dir_bin_path = buildings_dir.join("dir_bin");
    if !dir_bin_path.exists() {
        bail!("dir_bin file not found. Run extraction first.");
    }

    info!("Reading placement data from dir_bin...");
    let entries = dir_bin::DirBinReader::read_all(&dir_bin_path)
        .context("Failed to read dir_bin")?;

    info!("Found {} placement entries", entries.len());

    // Group entries by map
    let mut map_entries: std::collections::HashMap<u32, Vec<dir_bin::DirBinEntry>> =
        std::collections::HashMap::new();

    for entry in entries {
        // Apply filter if specified
        if !filter.is_empty() && !filter.contains(&entry.map_id) {
            continue;
        }
        map_entries.entry(entry.map_id).or_insert_with(Vec::new).push(entry);
    }

    info!("Processing {} maps", map_entries.len());

    // Collect all entries for .vmo conversion
    let all_entries: Vec<&dir_bin::DirBinEntry> = map_entries.values()
        .flat_map(|v| v.iter())
        .collect();

    // Convert raw model files to server-compatible .vmo format
    info!("Converting raw models to .vmo format...");
    match vmo_converter::convert_all_models(
        &all_entries.iter().map(|e| (*e).clone()).collect::<Vec<_>>(),
        &buildings_dir,
        &vmaps_dir,
    ) {
        Ok(count) => info!("  ✓ Converted {} models to .vmo format", count),
        Err(e) => warn!("  ✗ Error during .vmo conversion: {}", e),
    }

    // Process each map
    for (map_id, entries) in map_entries.iter() {
        info!("Assembling map {} ({} placements)", map_id, entries.len());

        // Build BIH tree for this map
        match tree::builder::build_map_tree(*map_id, entries, &buildings_dir) {
            Ok(tree_path) => {
                info!("  ✓ Built tree: {}", tree_path.display());
            }
            Err(e) => {
                warn!("  ✗ Failed to build tree for map {}: {}", map_id, e);
            }
        }
    }

    info!("✓ VMap assembly completed successfully");
    Ok(())
}
