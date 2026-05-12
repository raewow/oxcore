//! Unified WoW Data Extractor
//!
//! Extracts game data from World of Warcraft client for use by private servers.
//!
//! Supports:
//! - DBC files (database client files)
//! - Maps (terrain height and liquid data)
//! - Cameras (cinematic camera files)
//! - VMaps (3D geometry for collision/LOS)
//! - MMaps (navigation meshes for pathfinding)

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

mod dbc;
mod maps;
mod cameras;
mod shared;
mod vmaps;
mod mmaps;

/// Unified WoW Data Extractor
#[derive(Parser, Debug)]
#[command(name = "extractor")]
#[command(about = "Extract game data from World of Warcraft client", long_about = None)]
#[command(version)]
struct Cli {
    /// Input path (WoW Data directory or client root)
    #[arg(short = 'i', long = "input", global = true)]
    input: Option<PathBuf>,

    /// Output directory for extracted files
    /// Note: DBC files will be placed in <output>/dbc/
    /// For the server, use: -o ../server/data (files will go to server/data/dbc/)
    #[arg(short = 'o', long = "output", default_value = "./output", global = true)]
    output: PathBuf,

    /// Enable verbose logging
    #[arg(short = 'v', long = "verbose", global = true)]
    verbose: bool,

    /// Number of threads to use (0 = auto)
    #[arg(short = 'j', long = "jobs", default_value = "0", global = true)]
    jobs: usize,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Extract all data (DBC, Maps, Cameras, VMaps, MMaps)
    All {
        /// Skip DBC extraction
        #[arg(long)]
        skip_dbc: bool,

        /// Skip map extraction
        #[arg(long)]
        skip_maps: bool,

        /// Skip camera extraction
        #[arg(long)]
        skip_cameras: bool,

        /// Skip VMap extraction
        #[arg(long)]
        skip_vmaps: bool,

        /// Skip MMap generation
        #[arg(long)]
        skip_mmaps: bool,
    },

    /// Extract DBC (Database Client) files
    Dbc {
        /// List of specific DBC files to extract (default: all)
        #[arg(value_name = "FILE")]
        files: Vec<String>,
    },

    /// Extract map terrain data
    Maps {
        /// Enable float-to-int compression
        #[arg(short = 'c', long = "compress")]
        compress: bool,

        /// List of specific map IDs to extract (default: all)
        #[arg(value_name = "MAP_ID")]
        maps: Vec<u32>,
    },

    /// Extract camera files
    Cameras,

    /// Extract VMap geometry data
    Vmaps {
        /// Skip extraction, only assemble existing data
        #[arg(short = 'a', long = "assemble-only")]
        assemble_only: bool,

        /// Skip model extraction, only extract placement data (faster for testing)
        #[arg(short = 'p', long = "placement-only")]
        placement_only: bool,

        /// List of specific map IDs to extract (default: all)
        #[arg(value_name = "MAP_ID")]
        maps: Vec<u32>,
    },

    /// Generate navigation meshes (MMaps)
    Mmaps {
        /// List of specific map IDs to generate (default: all)
        #[arg(value_name = "MAP_ID")]
        maps: Vec<u32>,

        /// Generate debug meshes (.obj files)
        #[arg(long)]
        debug_meshes: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Validate input is provided
    let input = cli.input.ok_or_else(|| anyhow::anyhow!("Input path is required. Use -i/--input to specify the WoW Data directory"))?;

    // Initialize logging
    let level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(level.into())
                .add_directive("wow_mpq=warn".parse().unwrap()) // Suppress noisy MPQ attribute logs
        )
        .init();

    // Set up thread pool
    if cli.jobs > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(cli.jobs)
            .build_global()
            .unwrap();
    }

    info!("╔═══════════════════════════════════════╗");
    info!("║  WoW Data Extractor - Unified Tool   ║");
    info!("╚═══════════════════════════════════════╝");
    info!("");
    info!("Input:  {}", input.display());
    info!("Output: {}", cli.output.display());
    info!("");

    // Validate input path
    if !input.exists() {
        anyhow::bail!("Input path does not exist: {}", input.display());
    }

    // Create output directory
    std::fs::create_dir_all(&cli.output)?;

    // Execute command
    match cli.command {
        Commands::All {
            skip_dbc,
            skip_maps,
            skip_cameras,
            skip_vmaps,
            skip_mmaps,
        } => {
            extract_all(
                &input,
                &cli.output,
                !skip_dbc,
                !skip_maps,
                !skip_cameras,
                !skip_vmaps,
                !skip_mmaps,
            )?;
        }

        Commands::Dbc { files } => {
            dbc::extract(&input, &cli.output, files)?;
        }

        Commands::Maps { compress, maps } => {
            maps::extract(&input, &cli.output, compress, maps)?;
        }

        Commands::Cameras => {
            cameras::extract(&input, &cli.output)?;
        }

        Commands::Vmaps {
            assemble_only,
            placement_only,
            maps: map_ids,
        } => {
            vmaps::extract(&input, &cli.output, assemble_only, placement_only, map_ids)?;
        }

        Commands::Mmaps {
            maps: map_ids,
            debug_meshes,
        } => {
            mmaps::generate(&input, &cli.output, map_ids, debug_meshes)?;
        }
    }

    info!("");
    info!("✓ Extraction complete!");

    Ok(())
}

/// Extract all data types
fn extract_all(
    input: &PathBuf,
    output: &PathBuf,
    do_dbc: bool,
    do_maps: bool,
    do_cameras: bool,
    do_vmaps: bool,
    do_mmaps: bool,
) -> Result<()> {
    info!("═══ Extracting All Data ═══");
    info!("");

    let mut steps = Vec::new();
    if do_dbc {
        steps.push("DBC");
    }
    if do_maps {
        steps.push("Maps");
    }
    if do_cameras {
        steps.push("Cameras");
    }
    if do_vmaps {
        steps.push("VMaps");
    }
    if do_mmaps {
        steps.push("MMaps");
    }

    info!("Pipeline: {}", steps.join(" → "));
    info!("");

    // Step 1: Extract DBC files (required for other steps)
    if do_dbc {
        info!("┌─ Step 1: DBC Extraction");
        dbc::extract(input, output, Vec::new())?;
        info!("└─ DBC extraction complete");
        info!("");
    }

    // Step 2: Extract Maps
    if do_maps {
        info!("┌─ Step 2: Map Extraction");
        maps::extract(input, output, false, Vec::new())?;
        info!("└─ Map extraction complete");
        info!("");
    }

    // Step 3: Extract Cameras
    if do_cameras {
        info!("┌─ Step 3: Camera Extraction");
        cameras::extract(input, output)?;
        info!("└─ Camera extraction complete");
        info!("");
    }

    // Step 4: Extract and Assemble VMaps
    if do_vmaps {
        info!("┌─ Step 4: VMap Extraction & Assembly");
        vmaps::extract(input, output, false, false, Vec::new())?;
        info!("└─ VMap extraction complete");
        info!("");
    }

    // Step 5: Generate MMaps (requires maps + vmaps)
    if do_mmaps {
        if !do_maps || !do_vmaps {
            info!("⚠ Warning: MMap generation requires both Maps and VMaps");
            info!("  Skipping MMap generation");
        } else {
            info!("┌─ Step 5: MMap Generation");
            mmaps::generate(input, output, Vec::new(), false)?;
            info!("└─ MMap generation complete");
            info!("");
        }
    }

    Ok(())
}
