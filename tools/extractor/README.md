# WoW Data Extractor - Unified Tool

A unified command-line tool for extracting game data from World of Warcraft client files for use by private servers.

## Features

- **DBC Extraction** - Database client files
- **Map Extraction** - Terrain height maps and liquid data
- **Camera Extraction** - Cinematic camera files
- **VMap Extraction** - 3D geometry for collision and line-of-sight
- **MMap Generation** - Navigation meshes for NPC pathfinding

## Usage

### Extract Everything
```bash
# Extract all data types
extractor all -i "C:\Games\WoW\Data" -o "./output"

# Skip specific steps
extractor all -i "C:\Games\WoW\Data" -o "./output" --skip-mmaps
```

### Extract Specific Data Types
```bash
# Extract only DBC files
extractor dbc -i "C:\Games\WoW\Data" -o "./output"

# Extract only maps (with compression)
extractor maps -i "C:\Games\WoW\Data" -o "./output" --compress

# Extract specific maps only
extractor maps -i "C:\Games\WoW\Data" -o "./output" 0 1 530

# Extract VMaps (3D geometry for collision/line-of-sight)
extractor vmaps -i "C:\Games\WoW" -o "./output"

# Extract specific maps only
extractor vmaps -i "C:\Games\WoW" -o "./output" 0 1 530

# Assembly mode (skip extraction, build trees from existing data)
extractor vmaps -a -i "C:\Games\WoW" -o "./output"

# Generate MMaps (requires maps and vmaps)
extractor mmaps -i "C:\Games\WoW" -o "./output" --debug-meshes
```

### Options
```bash
-i, --input <PATH>      Input path (WoW Data directory)
-o, --output <PATH>     Output directory (default: ./output)
-v, --verbose           Enable verbose logging
-j, --jobs <N>          Number of threads (0 = auto)
```

## Architecture

```
extractor/
├── src/
│   ├── main.rs           # CLI with subcommands
│   ├── dbc/              # DBC extraction (✓ implemented)
│   ├── maps/             # Map extraction (TODO)
│   ├── cameras/          # Camera extraction (TODO)
│   ├── vmaps/            # VMap extraction (TODO)
│   ├── mmaps/            # MMap generation (TODO)
│   └── shared/           # Common utilities
│       ├── mpq.rs        # MPQ archive reading
│       ├── dbc_parser.rs # DBC file parsing
│       ├── config.rs     # Configuration
│       └── formats/      # File format definitions
│           ├── adt.rs    # ADT structures
│           ├── wdt.rs    # WDT structures
│           └── map_file.rs # Output format
```

## Status

- [x] Project structure
- [x] CLI with subcommands
- [x] DBC extraction (fully functional)
- [x] Map extraction (implemented, needs full ADT converter)
- [x] Camera extraction (fully functional)
- [x] VMap extraction (fully functional - WMO, M2, gameobjects)
- [ ] VMap assembly (BVH tree building for tiles)
- [ ] MMap generation

### Implementation Details

**DBC Extraction** ✓
- Loads MPQ archives from WoW Data directory
- Extracts all .dbc files to `output/dbc/`
- Supports filtering by filename
- Handles multiple MPQ archives and patches

**Map Extraction** ⚙️
- Reads Map.dbc to get map list
- Loads WDT files to find which tiles exist
- Extracts ADT terrain data
- Converts to custom .map binary format
- Supports map ID filtering
- TODO: Full ADT converter implementation (height/liquid/area extraction)

**Camera Extraction** ✓
- Reads CinematicCamera.dbc
- Extracts camera model files to `output/Cameras/`
- Handles all camera references from DBC

**VMap Extraction** ✓
- Extracts 3D geometry for collision detection and line-of-sight
- **WMO (World Map Objects)**: Buildings, dungeons, caves
  - Parses WMO root and group files
  - Converts geometry to VMAP binary format
  - High-precision vertex data extraction
- **M2 Models (Doodads)**: Trees, decorations, interactive objects
  - Parses M2 model files (.m2 and .mdx formats)
  - Extracts collision geometry
  - Supports both map-placed and gameobject models
- **Gameobject Models**: Chests, doors, interactive objects
  - Reads GameObjectDisplayInfo.dbc
  - Automatically extracts all GO M2 models
- **Custom WDT/ADT Parsers**: Lightweight, no external dependencies
  - Efficiently scans maps for model references
  - Only processes tiles that exist
- **Output**: `output/vmaps/Buildings/` directory
  - Root WMO files (.wmo)
  - WMO group files (_000.wmo, _001.wmo, etc.)
  - M2 model files
  - All in VMAP binary format ready for server use
- **Features**:
  - Progress bars with real-time feedback
  - Map filtering (extract specific maps only)
  - Graceful error handling (skips failed models)
  - Assembly mode for BVH tree building (TODO)

## Build Requirements

- Rust 1.70+
- For Windows: GCC (MinGW) or MSVC toolchain
  - The bundled SQLite feature requires a C compiler

## Building

```bash
cd tools/extractor
cargo build --release
```

The binary will be in `target/release/extractor.exe` (Windows) or `target/release/extractor` (Linux/Mac).

## License

GPL-2.0 (compatible with MaNGOS)
