//! Camera File Extraction

use anyhow::{Context, Result};
use std::path::Path;
use tracing::{debug, info};

use crate::shared::dbc_parser::DBCFile;
use crate::shared::mpq::MpqArchive;

/// Extract camera files
pub fn extract(input: &Path, output: &Path) -> Result<()> {
    info!("Extracting camera files...");

    // Load MPQ archives
    let mut archives = load_mpq_archives(input)?;

    // Read CinematicCamera.dbc
    let camera_dbc_path = output.join("dbc/CinematicCamera.dbc");
    if !camera_dbc_path.exists() {
        info!("CinematicCamera.dbc not found, skipping camera extraction");
        return Ok(());
    }

    let camera_dbc = DBCFile::open(&camera_dbc_path)?;
    debug!("Found {} camera(s)", camera_dbc.get_record_count());

    // Create cameras output directory
    let cameras_output = output.join("Cameras");
    std::fs::create_dir_all(&cameras_output)
        .with_context(|| format!("Failed to create directory: {}", cameras_output.display()))?;

    // Extract each camera file
    let mut extracted_count = 0;
    for record in camera_dbc.iter() {
        let camera_id = record.get_uint(0);
        let model_path = record.get_string(1);

        if model_path.is_empty() {
            continue;
        }

        debug!("Extracting camera {}: {}", camera_id, model_path);

        // Try to extract camera file
        for archive in &mut archives {
            // Try both slash types
            for path in &[model_path, &model_path.replace('\\', "/")] {
                if let Ok(data) = archive.archive.read_file(path) {
                    let filename = path.rsplit(['\\', '/']).next().unwrap_or(path);
                    let output_file = cameras_output.join(filename);

                    std::fs::write(&output_file, data)
                        .with_context(|| format!("Failed to write: {}", output_file.display()))?;

                    extracted_count += 1;
                    debug!("Extracted: {}", filename);
                    break;
                }
            }
        }
    }

    info!("✓ Extracted {} camera file(s)", extracted_count);

    Ok(())
}

/// Load MPQ archives from the input directory
fn load_mpq_archives(input: &Path) -> Result<Vec<MpqArchive>> {
    let mut archives = Vec::new();

    let mpq_files = [
        "model.MPQ",
        "dbc.MPQ",
        "patch.MPQ",
        "patch-2.MPQ",
        "patch-3.MPQ",
    ];

    let data_dir = input.join("Data");
    if !data_dir.exists() {
        return Ok(archives);
    }

    for mpq_name in &mpq_files {
        let mpq_path = data_dir.join(mpq_name);
        if mpq_path.exists() {
            if let Ok(archive) = MpqArchive::open(&mpq_path) {
                archives.push(archive);
                debug!("Loaded: {}", mpq_name);
            }
        }
    }

    info!("Loaded {} MPQ archive(s)", archives.len());
    Ok(archives)
}
