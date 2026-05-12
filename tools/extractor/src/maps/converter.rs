//! ADT to Map File Converter
//!
//! Converts ADT (terrain) files to custom .map binary format
//! Based on MaNGOS/vmangos extractor implementation

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use tracing::debug;

use crate::shared::config::ExtractorConfig;
use crate::shared::formats::adt::{ADTFile, ADT_CELLS_PER_GRID, ADT_CELL_SIZE, ADT_GRID_SIZE, V8_SIZE, V9_SIZE};
use crate::shared::formats::map_file::*;

/// Convert ADT data to map file
pub fn convert_adt(
    adt_data: &[u8],
    output_path: &Path,
    _tile_x: u32,
    _tile_y: u32,
    config: &ExtractorConfig,
) -> Result<()> {
    debug!("Converting ADT to {}", output_path.display());

    // Parse ADT file
    let adt = ADTFile::from_bytes(adt_data.to_vec())
        .with_context(|| "Failed to parse ADT file")?;

    // Extract data from ADT
    let area_data = extract_area_data(&adt)?;
    let height_data = extract_height_data(&adt, config)?;
    let liquid_data = extract_liquid_data(&adt, config)?;
    let holes_data = extract_holes_data(&adt)?;

    // Write output file
    write_map_file(output_path, area_data, height_data, liquid_data, holes_data)?;

    Ok(())
}

/// Extract area flags from ADT
/// Based on MaNGOS System.cpp lines 292-343
fn extract_area_data(adt: &ADTFile) -> Result<(GridMapAreaHeader, AreaData)> {
    // Get area IDs from all MCNK chunks
    let mut area_ids = [[0u16; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID];

    // Extract area ID from each chunk
    for (idx, chunk) in adt.chunks.iter().enumerate() {
        let y = idx / ADT_CELLS_PER_GRID;
        let x = idx % ADT_CELLS_PER_GRID;
        if y < ADT_CELLS_PER_GRID && x < ADT_CELLS_PER_GRID {
            // For now, store raw area_id as u16
            // TODO: In full implementation, this would lookup area flags from AreaTable.dbc
            area_ids[y][x] = if chunk.mcnk.area_id > 0 && chunk.mcnk.area_id <= 0xFFFF {
                chunk.mcnk.area_id as u16
            } else {
                0xFFFF // Invalid area
            };
        }
    }

    // Try to pack area data - check if all cells have same area
    let first_area = area_ids[0][0];
    let mut all_same = true;
    for y in 0..ADT_CELLS_PER_GRID {
        for x in 0..ADT_CELLS_PER_GRID {
            if area_ids[y][x] != first_area {
                all_same = false;
                break;
            }
        }
        if !all_same {
            break;
        }
    }

    let header = GridMapAreaHeader {
        fourcc: u32::from_le_bytes(*MAP_AREA_MAGIC),
        flags: if all_same { MAP_AREA_NO_AREA } else { 0 },
        grid_area: if all_same { first_area } else { 0 },
    };

    let data = if all_same {
        AreaData::Single(first_area)
    } else {
        AreaData::Grid(area_ids)
    };

    Ok((header, data))
}

/// Extract height data from ADT
/// Based on MaNGOS System.cpp lines 348-515
fn extract_height_data(
    adt: &ADTFile,
    config: &ExtractorConfig,
) -> Result<(GridMapHeightHeader, HeightData)> {
    // Initialize height grids on heap to avoid stack overflow
    // V9 is 129x129 (vertices at cell corners)
    // V8 is 128x128 (vertices at cell centers)
    let mut v9 = Box::new([[0.0f32; V9_SIZE]; V9_SIZE]);
    let mut v8 = Box::new([[0.0f32; V8_SIZE]; V8_SIZE]);

    // Extract height from each MCNK chunk
    for chunk in &adt.chunks {
        let i = chunk.mcnk.iy as usize;
        let j = chunk.mcnk.ix as usize;

        if i >= ADT_CELLS_PER_GRID || j >= ADT_CELLS_PER_GRID {
            continue;
        }

        // Set base height from chunk position (ypos)
        // MaNGOS: V9[cy][cx] = cell->ypos
        for y in 0..=ADT_CELL_SIZE {
            let cy = i * ADT_CELL_SIZE + y;
            for x in 0..=ADT_CELL_SIZE {
                let cx = j * ADT_CELL_SIZE + x;
                if cy < V9_SIZE && cx < V9_SIZE {
                    v9[cy][cx] = chunk.mcnk.position[2]; // ypos is position[2] in WoW coords
                }
            }
        }

        for y in 0..ADT_CELL_SIZE {
            let cy = i * ADT_CELL_SIZE + y;
            for x in 0..ADT_CELL_SIZE {
                let cx = j * ADT_CELL_SIZE + x;
                if cy < V8_SIZE && cx < V8_SIZE {
                    v8[cy][cx] = chunk.mcnk.position[2];
                }
            }
        }

        // Add height from MCVT if present
        if let Some(ref mcvt) = chunk.mcvt {
            // Extract V9 height map (9x9 corners)
            // MaNGOS: V9[cy][cx] += v->height_map[y * (ADT_CELL_SIZE * 2 + 1) + x]
            for y in 0..=ADT_CELL_SIZE {
                let cy = i * ADT_CELL_SIZE + y;
                for x in 0..=ADT_CELL_SIZE {
                    let cx = j * ADT_CELL_SIZE + x;
                    if cy < V9_SIZE && cx < V9_SIZE {
                        let idx = y * (ADT_CELL_SIZE * 2 + 1) + x;
                        if idx < mcvt.heights.len() {
                            v9[cy][cx] += mcvt.heights[idx];
                        }
                    }
                }
            }

            // Extract V8 height map (8x8 centers)
            // MaNGOS: V8[cy][cx] += v->height_map[y * (ADT_CELL_SIZE * 2 + 1) + ADT_CELL_SIZE + 1 + x]
            for y in 0..ADT_CELL_SIZE {
                let cy = i * ADT_CELL_SIZE + y;
                for x in 0..ADT_CELL_SIZE {
                    let cx = j * ADT_CELL_SIZE + x;
                    if cy < V8_SIZE && cx < V8_SIZE {
                        let idx = y * (ADT_CELL_SIZE * 2 + 1) + ADT_CELL_SIZE + 1 + x;
                        if idx < mcvt.heights.len() {
                            v8[cy][cx] += mcvt.heights[idx];
                        }
                    }
                }
            }
        }
    }

    // Find min/max heights (MaNGOS lines 419-438)
    let mut min_height = 20000.0f32;
    let mut max_height = -20000.0f32;

    for y in 0..V8_SIZE {
        for x in 0..V8_SIZE {
            let h = v8[y][x];
            if h < min_height {
                min_height = h;
            }
            if h > max_height {
                max_height = h;
            }
        }
    }

    for y in 0..V9_SIZE {
        for x in 0..V9_SIZE {
            let h = v9[y][x];
            if h < min_height {
                min_height = h;
            }
            if h > max_height {
                max_height = h;
            }
        }
    }

    // Apply height limit if configured (MaNGOS lines 441-455)
    if config.allow_height_limit && min_height < config.use_min_height {
        for y in 0..V8_SIZE {
            for x in 0..V8_SIZE {
                if v8[y][x] < config.use_min_height {
                    v8[y][x] = config.use_min_height;
                }
            }
        }
        for y in 0..V9_SIZE {
            for x in 0..V9_SIZE {
                if v9[y][x] < config.use_min_height {
                    v9[y][x] = config.use_min_height;
                }
            }
        }
        if min_height < config.use_min_height {
            min_height = config.use_min_height;
        }
        if max_height < config.use_min_height {
            max_height = config.use_min_height;
        }
    }

    let mut header = GridMapHeightHeader {
        fourcc: u32::from_le_bytes(*MAP_HEIGHT_MAGIC),
        flags: 0,
        grid_height: min_height,
        grid_max_height: max_height,
    };

    // Check if height data is needed (MaNGOS lines 466-471)
    if max_height == min_height {
        header.flags |= MAP_HEIGHT_NO_HEIGHT;
        return Ok((header, HeightData::None));
    }

    let diff = max_height - min_height;
    if config.allow_float_to_int && diff < config.flat_height_delta_limit {
        header.flags |= MAP_HEIGHT_NO_HEIGHT;
        return Ok((header, HeightData::None));
    }

    // Try compression (MaNGOS lines 474-516)
    if config.allow_float_to_int {
        // Try uint8 compression
        if diff < config.float_to_int8_limit {
            header.flags |= MAP_HEIGHT_AS_INT8;
            let step = select_uint8_step(diff);

            let mut v9_u8 = Box::new([[0u8; V9_SIZE]; V9_SIZE]);
            let mut v8_u8 = Box::new([[0u8; V8_SIZE]; V8_SIZE]);

            for y in 0..V8_SIZE {
                for x in 0..V8_SIZE {
                    v8_u8[y][x] = ((v8[y][x] - min_height) * step + 0.5) as u8;
                }
            }
            for y in 0..V9_SIZE {
                for x in 0..V9_SIZE {
                    v9_u8[y][x] = ((v9[y][x] - min_height) * step + 0.5) as u8;
                }
            }

            return Ok((header, HeightData::UInt8 { v9: v9_u8, v8: v8_u8 }));
        }
        // Try uint16 compression
        else if diff < config.float_to_int16_limit {
            header.flags |= MAP_HEIGHT_AS_INT16;
            let step = select_uint16_step(diff);

            let mut v9_u16 = Box::new([[0u16; V9_SIZE]; V9_SIZE]);
            let mut v8_u16 = Box::new([[0u16; V8_SIZE]; V8_SIZE]);

            for y in 0..V8_SIZE {
                for x in 0..V8_SIZE {
                    v8_u16[y][x] = ((v8[y][x] - min_height) * step + 0.5) as u16;
                }
            }
            for y in 0..V9_SIZE {
                for x in 0..V9_SIZE {
                    v9_u16[y][x] = ((v9[y][x] - min_height) * step + 0.5) as u16;
                }
            }

            return Ok((header, HeightData::UInt16 { v9: v9_u16, v8: v8_u16 }));
        }
    }

    // Store as float (no compression)
    Ok((header, HeightData::Float { v9, v8 }))
}

/// Select compression step for uint8 storage
fn select_uint8_step(max_diff: f32) -> f32 {
    255.0 / max_diff
}

/// Select compression step for uint16 storage
fn select_uint16_step(max_diff: f32) -> f32 {
    65535.0 / max_diff
}

/// Extract liquid data from ADT
/// Based on MaNGOS System.cpp lines 518-667
fn extract_liquid_data(
    adt: &ADTFile,
    config: &ExtractorConfig,
) -> Result<Option<(GridMapLiquidHeader, LiquidData)>> {
    // Liquid data arrays
    let mut liquid_entry = [[0u16; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID];
    let mut liquid_flags = [[0u8; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID];
    let mut liquid_show = [[false; ADT_GRID_SIZE]; ADT_GRID_SIZE];
    let mut liquid_height = [[0.0f32; ADT_GRID_SIZE + 1]; ADT_GRID_SIZE + 1];

    // Extract liquid data from MCLQ chunks (vanilla/TBC format)
    for chunk in &adt.chunks {
        let i = chunk.mcnk.iy as usize;
        let j = chunk.mcnk.ix as usize;

        if i >= ADT_CELLS_PER_GRID || j >= ADT_CELLS_PER_GRID {
            continue;
        }

        // Get MCLQ data if present
        if let Some(ref mclq) = chunk.mclq {
            // Check each cell for liquid visibility
            for y in 0..ADT_CELL_SIZE {
                let cy = i * ADT_CELL_SIZE + y;
                for x in 0..ADT_CELL_SIZE {
                    let cx = j * ADT_CELL_SIZE + x;

                    // 0x0F means no liquid
                    if mclq.flags[y][x] != 0x0F {
                        liquid_show[cy][cx] = true;

                        // Check for deep water flag (bit 7)
                        if mclq.flags[y][x] & (1 << 7) != 0 {
                            liquid_flags[i][j] |= MAP_LIQUID_TYPE_DEEP_WATER;
                        }
                    }
                }
            }

            // Determine liquid type from MCNK flags
            let c_flag = chunk.mcnk.flags;
            if c_flag & (1 << 2) != 0 {
                liquid_entry[i][j] = 1;
                liquid_flags[i][j] |= MAP_LIQUID_TYPE_WATER;
            }
            if c_flag & (1 << 3) != 0 {
                liquid_entry[i][j] = 2;
                liquid_flags[i][j] |= MAP_LIQUID_TYPE_OCEAN;
            }
            if c_flag & (1 << 4) != 0 {
                liquid_entry[i][j] = 3;
                liquid_flags[i][j] |= MAP_LIQUID_TYPE_MAGMA;
            }

            // Extract liquid heights (9x9 grid for each cell)
            for y in 0..=ADT_CELL_SIZE {
                let cy = i * ADT_CELL_SIZE + y;
                for x in 0..=ADT_CELL_SIZE {
                    let cx = j * ADT_CELL_SIZE + x;
                    if cy < liquid_height.len() && cx < liquid_height[0].len() {
                        liquid_height[cy][cx] = mclq.liquid[y][x].height;
                    }
                }
            }
        }
    }

    // Check if we have any liquid data
    let first_liquid_entry = liquid_entry[0][0];
    let first_liquid_flag = liquid_flags[0][0];

    // Check if all cells have the same type (can be packed)
    let mut full_type = false;
    for y in 0..ADT_CELLS_PER_GRID {
        for x in 0..ADT_CELLS_PER_GRID {
            if liquid_entry[y][x] != first_liquid_entry || liquid_flags[y][x] != first_liquid_flag {
                full_type = true;
                break;
            }
        }
        if full_type {
            break;
        }
    }

    // No liquid data if all cells have no liquid type
    if first_liquid_flag == 0 && !full_type {
        return Ok(None);
    }

    // Find bounding box of visible liquid and min/max heights
    let mut min_x = 255usize;
    let mut min_y = 255usize;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut max_height = -20000.0f32;
    let mut min_height = 20000.0f32;

    for y in 0..ADT_GRID_SIZE {
        for x in 0..ADT_GRID_SIZE {
            if liquid_show[y][x] {
                if min_x > x { min_x = x; }
                if max_x < x { max_x = x; }
                if min_y > y { min_y = y; }
                if max_y < y { max_y = y; }

                let h = liquid_height[y][x];
                if max_height < h { max_height = h; }
                if min_height > h { min_height = h; }
            } else {
                // Set non-visible cells to minimum height
                liquid_height[y][x] = config.use_min_height;
                if min_height > config.use_min_height {
                    min_height = config.use_min_height;
                }
            }
        }
    }

    // Build liquid header
    let mut header = GridMapLiquidHeader {
        fourcc: u32::from_le_bytes(*MAP_LIQUID_MAGIC),
        flags: 0,
        liquid_type: 0,
        liquid_flags: 0,
        offset_x: min_x as u8,
        offset_y: min_y as u8,
        width: (max_x - min_x + 1 + 1) as u8,
        height: (max_y - min_y + 1 + 1) as u8,
        liquid_level: min_height,
    };

    // Check if height data is needed
    if max_height == min_height {
        header.flags |= MAP_LIQUID_NO_HEIGHT;
    }

    // Not needed if flat surface
    if config.allow_float_to_int && (max_height - min_height) < config.flat_liquid_delta_limit {
        header.flags |= MAP_LIQUID_NO_HEIGHT;
    }

    // Check if type data is needed
    if !full_type {
        header.flags |= MAP_LIQUID_NO_TYPE;
        header.liquid_flags = first_liquid_flag;
        header.liquid_type = first_liquid_entry;
    }

    // Build liquid data
    let height_data = if header.flags & MAP_LIQUID_NO_HEIGHT == 0 {
        // Extract only the visible region
        let mut heights = [[0.0f32; ADT_GRID_SIZE + 1]; ADT_GRID_SIZE + 1];
        for y in 0..header.height as usize {
            for x in 0..header.width as usize {
                let src_y = y + header.offset_y as usize;
                let src_x = x + header.offset_x as usize;
                if src_y < liquid_height.len() && src_x < liquid_height[0].len() {
                    heights[y][x] = liquid_height[src_y][src_x];
                }
            }
        }
        Some(heights)
    } else {
        None
    };

    let data = LiquidData {
        entry: liquid_entry,
        flags: liquid_flags,
        height: height_data,
    };

    Ok(Some((header, data)))
}

/// Extract holes data from ADT
/// Based on MaNGOS System.cpp lines 669-689
fn extract_holes_data(adt: &ADTFile) -> Result<Option<HolesData>> {
    let mut holes = [[0u16; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID];
    let mut has_holes = false;

    for chunk in &adt.chunks {
        let i = chunk.mcnk.iy as usize;
        let j = chunk.mcnk.ix as usize;

        if i < ADT_CELLS_PER_GRID && j < ADT_CELLS_PER_GRID {
            holes[i][j] = chunk.mcnk.holes;
            if chunk.mcnk.holes != 0 {
                has_holes = true;
            }
        }
    }

    if has_holes {
        Ok(Some(holes))
    } else {
        Ok(None)
    }
}

/// Write map file to disk
fn write_map_file(
    output_path: &Path,
    (area_header, area_data): (GridMapAreaHeader, AreaData),
    (height_header, height_data): (GridMapHeightHeader, HeightData),
    liquid_data: Option<(GridMapLiquidHeader, LiquidData)>,
    holes_data: Option<HolesData>,
) -> Result<()> {
    let file = File::create(output_path)
        .with_context(|| format!("Failed to create file: {}", output_path.display()))?;

    let mut writer = BufWriter::new(file);

    // Calculate offsets and sizes
    let header_size = 40; // GridMapFileHeader size

    // Area header: fourcc(4) + flags(2) + grid_area(2) = 8 bytes
    let area_header_size = 8;
    let area_data_size = area_data.size();
    let area_map_size = area_header_size + area_data_size;

    // Height header: fourcc(4) + flags(4) + grid_height(4) + grid_max_height(4) = 16 bytes
    let height_header_size = 16;
    let height_data_size = height_data.size();
    let height_map_size = height_header_size + height_data_size;

    // Liquid header: fourcc(4) + flags(1) + liquidFlags(1) + liquidType(2) +
    //                offsetX(1) + offsetY(1) + width(1) + height(1) + liquidLevel(4) = 16 bytes
    let (liquid_header_size, liquid_data_size) = if let Some((_, ref data)) = liquid_data {
        (16, data.size())
    } else {
        (0, 0)
    };
    let liquid_map_size = liquid_header_size + liquid_data_size;

    let holes_size = if holes_data.is_some() {
        holes_size()
    } else {
        0
    };

    // Calculate offsets
    let area_map_offset = header_size;
    let height_map_offset = area_map_offset + area_map_size as u32;
    let liquid_map_offset = height_map_offset + height_map_size as u32;
    let holes_offset = liquid_map_offset + liquid_map_size as u32;

    // Write main header
    let file_header = GridMapFileHeader {
        map_magic: u32::from_le_bytes(*MAP_MAGIC),
        version_magic: u32::from_le_bytes(*MAP_VERSION_MAGIC),
        area_map_offset,
        area_map_size: area_map_size as u32,
        height_map_offset,
        height_map_size: height_map_size as u32,
        liquid_map_offset,
        liquid_map_size: liquid_map_size as u32,
        holes_offset,
        holes_size: holes_size as u32,
    };

    file_header.write(&mut writer)?;

    // Write area map
    area_header.write(&mut writer)?;
    area_data.write(&mut writer)?;

    // Write height map
    height_header.write(&mut writer)?;
    height_data.write(&mut writer)?;

    // Write liquid map (if present)
    if let Some((liquid_header, liquid_data)) = liquid_data {
        liquid_header.write(&mut writer)?;
        liquid_data.write(&mut writer)?;
    }

    // Write holes (if present)
    if let Some(holes) = holes_data {
        write_holes(&mut writer, &holes)?;
    }

    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_area_data() {
        // Create minimal ADT
        let adt_data = create_minimal_adt();
        let adt = ADTFile::from_bytes(adt_data).unwrap();

        let (header, data) = extract_area_data(&adt).unwrap();

        assert_eq!(header.fourcc, u32::from_le_bytes(*MAP_AREA_MAGIC));
        // Since minimal ADT has no chunks, it should have NO_AREA flag
        assert_eq!(header.flags & MAP_AREA_NO_AREA, MAP_AREA_NO_AREA);

        match data {
            AreaData::Single(_) => {} // Expected for empty/uniform data
            _ => {}
        }
    }

    #[test]
    fn test_extract_height_data_no_compression() {
        let adt_data = create_minimal_adt();
        let adt = ADTFile::from_bytes(adt_data).unwrap();
        let config = ExtractorConfig::default();

        let (header, data) = extract_height_data(&adt, &config).unwrap();

        assert_eq!(header.fourcc, u32::from_le_bytes(*MAP_HEIGHT_MAGIC));
        // With no chunks, min == max, so NO_HEIGHT flag should be set
        assert_eq!(header.flags & MAP_HEIGHT_NO_HEIGHT, MAP_HEIGHT_NO_HEIGHT);

        match data {
            HeightData::None => {} // Expected
            _ => panic!("Expected None height data"),
        }
    }

    #[test]
    fn test_extract_height_data_with_compression() {
        let adt_data = create_minimal_adt();
        let adt = ADTFile::from_bytes(adt_data).unwrap();
        let config = ExtractorConfig::with_compression();

        let (header, _data) = extract_height_data(&adt, &config).unwrap();

        // With no chunks, should still have NO_HEIGHT flag
        assert!(header.flags & MAP_HEIGHT_NO_HEIGHT != 0);
    }

    #[test]
    fn test_extract_liquid_data() {
        let adt_data = create_minimal_adt();
        let adt = ADTFile::from_bytes(adt_data).unwrap();
        let config = ExtractorConfig::default();

        let result = extract_liquid_data(&adt, &config).unwrap();

        assert!(result.is_none()); // No liquid in minimal ADT
    }

    #[test]
    fn test_extract_holes_data() {
        let adt_data = create_minimal_adt();
        let adt = ADTFile::from_bytes(adt_data).unwrap();

        let result = extract_holes_data(&adt).unwrap();

        assert!(result.is_none()); // No holes in minimal ADT
    }

    #[test]
    fn test_write_map_file() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.map");

        let area_header = GridMapAreaHeader {
            fourcc: u32::from_le_bytes(*MAP_AREA_MAGIC),
            flags: MAP_AREA_NO_AREA,
            grid_area: 0,
        };

        let height_header = GridMapHeightHeader {
            fourcc: u32::from_le_bytes(*MAP_HEIGHT_MAGIC),
            flags: MAP_HEIGHT_NO_HEIGHT,
            grid_height: 0.0,
            grid_max_height: 0.0,
        };

        write_map_file(
            &output_path,
            (area_header, AreaData::Single(0)),
            (height_header, HeightData::None),
            None,
            None,
        )
        .unwrap();

        // Verify file was created
        assert!(output_path.exists());

        // Verify file has correct magic bytes
        let data = std::fs::read(&output_path).unwrap();
        assert!(data.len() >= 8);
        assert_eq!(&data[0..4], MAP_MAGIC);
        assert_eq!(&data[4..8], MAP_VERSION_MAGIC);
    }

    #[test]
    fn test_convert_adt_full() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("converted.map");

        let adt_data = create_minimal_adt();
        let config = ExtractorConfig::default();

        convert_adt(&adt_data, &output_path, 0, 0, &config).unwrap();

        // Verify file was created and has valid format
        assert!(output_path.exists());

        let data = std::fs::read(&output_path).unwrap();
        assert!(data.len() >= 8);
        assert_eq!(&data[0..4], MAP_MAGIC);
        assert_eq!(&data[4..8], MAP_VERSION_MAGIC);
    }

    #[test]
    fn test_compression_step_calculation() {
        // Test uint8 step
        let step8 = select_uint8_step(255.0);
        assert!((step8 - 1.0).abs() < 0.001);

        let step8_half = select_uint8_step(127.5);
        assert!((step8_half - 2.0).abs() < 0.001);

        // Test uint16 step
        let step16 = select_uint16_step(65535.0);
        assert!((step16 - 1.0).abs() < 0.001);
    }

    /// Create a minimal valid ADT file for testing
    fn create_minimal_adt() -> Vec<u8> {
        let mut data = Vec::new();

        // MHDR chunk
        data.extend_from_slice(b"MHDR");
        data.extend_from_slice(&64u32.to_le_bytes()); // size (must be at least 64)

        // Write minimal MHDR data (16 u32 fields to reach 64 bytes)
        for _ in 0..16 {
            data.extend_from_slice(&0u32.to_le_bytes());
        }

        data
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_area_data_size() {
        let single = AreaData::Single(123);
        assert_eq!(single.size(), 0); // Single area stored in header

        let grid = AreaData::Grid([[456; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID]);
        assert_eq!(grid.size(), ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID * 2);
    }

    #[test]
    fn test_height_data_size() {
        assert_eq!(HeightData::None.size(), 0);

        let float_data = HeightData::Float {
            v9: Box::new([[0.0; V9_SIZE]; V9_SIZE]),
            v8: Box::new([[0.0; V8_SIZE]; V8_SIZE]),
        };
        assert_eq!(float_data.size(), (V9_SIZE * V9_SIZE + V8_SIZE * V8_SIZE) * 4);

        let uint16_data = HeightData::UInt16 {
            v9: Box::new([[0; V9_SIZE]; V9_SIZE]),
            v8: Box::new([[0; V8_SIZE]; V8_SIZE]),
        };
        assert_eq!(uint16_data.size(), (V9_SIZE * V9_SIZE + V8_SIZE * V8_SIZE) * 2);

        let uint8_data = HeightData::UInt8 {
            v9: Box::new([[0; V9_SIZE]; V9_SIZE]),
            v8: Box::new([[0; V8_SIZE]; V8_SIZE]),
        };
        assert_eq!(uint8_data.size(), V9_SIZE * V9_SIZE + V8_SIZE * V8_SIZE);
    }
}
