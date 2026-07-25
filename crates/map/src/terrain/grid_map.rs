//! A single terrain grid loaded from `maps/{map:03}{gy:02}{gx:02}.map`.
//!
//! Ported from MaNGOS `GridMap` (GridMap.cpp). Holds the area ids, the height
//! mesh, and the liquid layer for one 533.33×533.33 grid.

use anyhow::{bail, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use super::defines::*;

/// Height storage variant used by the grid, mirroring the extractor's packing.
enum HeightMap {
    /// No height data — the whole grid is flat at `grid_height`.
    Flat,
    Float {
        v9: Vec<f32>,
        v8: Vec<f32>,
    },
    Uint16 {
        v9: Vec<u16>,
        v8: Vec<u16>,
    },
    Uint8 {
        v9: Vec<u8>,
        v8: Vec<u8>,
    },
}

/// Terrain data for one grid.
pub struct GridMap {
    // ---- Area ----
    /// Area id covering the whole grid, used when `area_map` is absent.
    grid_area: u16,
    /// Per-cell area ids (16×16).
    area_map: Option<Vec<u16>>,

    // ---- Height ----
    heights: HeightMap,
    /// Base height that packed integer heights are offset from.
    grid_height: f32,
    /// Scale applied to packed integer heights.
    height_multiplier: f32,
    /// ADT hole bitmap (16×16 cells), used to punch gaps in the height mesh.
    holes: Option<Vec<u16>>,

    // ---- Liquid ----
    /// Liquid type covering the whole grid, used when `liquid_flags` is absent.
    liquid_global_flags: u8,
    /// Liquid entry covering the whole grid, used when `liquid_entry` is absent.
    liquid_global_entry: u16,
    /// Per-cell liquid type flags (16×16).
    liquid_flags: Option<Vec<u8>>,
    /// Per-cell liquid entries (16×16).
    liquid_entry: Option<Vec<u16>>,
    /// Liquid surface heights, `liquid_width * liquid_height` samples.
    liquid_map: Option<Vec<f32>>,
    /// Surface height used when `liquid_map` is absent.
    liquid_level: f32,
    liquid_off_x: u8,
    liquid_off_y: u8,
    liquid_width: u8,
    liquid_height: u8,
}

/// Hole lookup tables from `GridMap.cpp`.
const HOLETAB_H: [u16; 4] = [0x1111, 0x2222, 0x4444, 0x8888];
const HOLETAB_V: [u16; 4] = [0x000F, 0x00F0, 0x0F00, 0xF000];

impl GridMap {
    /// Load a grid from disk.
    ///
    /// A missing file is not an error — most grids simply have no `.map` file.
    pub fn load(path: &Path) -> Result<Option<Self>> {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let mut cur = Cursor::new(&bytes[..]);

        let mut map_magic = [0u8; 4];
        let mut version_magic = [0u8; 4];
        cur.read_exact(&mut map_magic)?;
        cur.read_exact(&mut version_magic)?;
        if &map_magic != MAP_MAGIC || &version_magic != MAP_VERSION_MAGIC {
            bail!(
                "map file {:?} has incompatible magic {:?}/{:?} (re-extract required)",
                path,
                String::from_utf8_lossy(&map_magic),
                String::from_utf8_lossy(&version_magic)
            );
        }

        let area_offset = cur.read_u32::<LittleEndian>()?;
        let _area_size = cur.read_u32::<LittleEndian>()?;
        let height_offset = cur.read_u32::<LittleEndian>()?;
        let _height_size = cur.read_u32::<LittleEndian>()?;
        let liquid_offset = cur.read_u32::<LittleEndian>()?;
        let _liquid_size = cur.read_u32::<LittleEndian>()?;
        let holes_offset = cur.read_u32::<LittleEndian>()?;
        let _holes_size = cur.read_u32::<LittleEndian>()?;

        let mut grid = Self {
            grid_area: 0,
            area_map: None,
            heights: HeightMap::Flat,
            grid_height: INVALID_HEIGHT_VALUE,
            height_multiplier: 0.0,
            holes: None,
            liquid_global_flags: 0,
            liquid_global_entry: 0,
            liquid_flags: None,
            liquid_entry: None,
            liquid_map: None,
            liquid_level: INVALID_HEIGHT_VALUE,
            liquid_off_x: 0,
            liquid_off_y: 0,
            liquid_width: 0,
            liquid_height: 0,
        };

        if area_offset != 0 {
            grid.load_area(&mut cur, area_offset)?;
        }
        if holes_offset != 0 {
            grid.load_holes(&mut cur, holes_offset)?;
        }
        if height_offset != 0 {
            grid.load_height(&mut cur, height_offset)?;
        }
        if liquid_offset != 0 {
            grid.load_liquid(&mut cur, liquid_offset)?;
        }

        Ok(Some(grid))
    }

    fn load_area(&mut self, cur: &mut Cursor<&[u8]>, offset: u32) -> Result<()> {
        cur.seek(SeekFrom::Start(offset as u64))?;

        let mut fourcc = [0u8; 4];
        cur.read_exact(&mut fourcc)?;
        if &fourcc != MAP_AREA_MAGIC {
            bail!("bad area section magic");
        }

        let flags = cur.read_u16::<LittleEndian>()?;
        self.grid_area = cur.read_u16::<LittleEndian>()?;

        if flags & MAP_AREA_NO_AREA == 0 {
            let mut areas = vec![0u16; 16 * 16];
            cur.read_u16_into::<LittleEndian>(&mut areas)?;
            self.area_map = Some(areas);
        }

        Ok(())
    }

    fn load_holes(&mut self, cur: &mut Cursor<&[u8]>, offset: u32) -> Result<()> {
        cur.seek(SeekFrom::Start(offset as u64))?;
        let mut holes = vec![0u16; 16 * 16];
        cur.read_u16_into::<LittleEndian>(&mut holes)?;
        self.holes = Some(holes);
        Ok(())
    }

    fn load_height(&mut self, cur: &mut Cursor<&[u8]>, offset: u32) -> Result<()> {
        cur.seek(SeekFrom::Start(offset as u64))?;

        let mut fourcc = [0u8; 4];
        cur.read_exact(&mut fourcc)?;
        if &fourcc != MAP_HEIGHT_MAGIC {
            bail!("bad height section magic");
        }

        let flags = cur.read_u32::<LittleEndian>()?;
        self.grid_height = cur.read_f32::<LittleEndian>()?;
        let grid_max_height = cur.read_f32::<LittleEndian>()?;

        if flags & MAP_HEIGHT_NO_HEIGHT != 0 {
            self.heights = HeightMap::Flat;
            return Ok(());
        }

        if flags & MAP_HEIGHT_AS_INT16 != 0 {
            let mut v9 = vec![0u16; 129 * 129];
            let mut v8 = vec![0u16; 128 * 128];
            cur.read_u16_into::<LittleEndian>(&mut v9)?;
            cur.read_u16_into::<LittleEndian>(&mut v8)?;
            self.height_multiplier = (grid_max_height - self.grid_height) / 65535.0;
            self.heights = HeightMap::Uint16 { v9, v8 };
        } else if flags & MAP_HEIGHT_AS_INT8 != 0 {
            let mut v9 = vec![0u8; 129 * 129];
            let mut v8 = vec![0u8; 128 * 128];
            cur.read_exact(&mut v9)?;
            cur.read_exact(&mut v8)?;
            self.height_multiplier = (grid_max_height - self.grid_height) / 255.0;
            self.heights = HeightMap::Uint8 { v9, v8 };
        } else {
            let mut v9 = vec![0f32; 129 * 129];
            let mut v8 = vec![0f32; 128 * 128];
            cur.read_f32_into::<LittleEndian>(&mut v9)?;
            cur.read_f32_into::<LittleEndian>(&mut v8)?;
            self.heights = HeightMap::Float { v9, v8 };
        }

        Ok(())
    }

    fn load_liquid(&mut self, cur: &mut Cursor<&[u8]>, offset: u32) -> Result<()> {
        cur.seek(SeekFrom::Start(offset as u64))?;

        let mut fourcc = [0u8; 4];
        cur.read_exact(&mut fourcc)?;
        if &fourcc != MAP_LIQUID_MAGIC {
            bail!("bad liquid section magic");
        }

        let flags = cur.read_u8()?;
        self.liquid_global_flags = cur.read_u8()?;
        self.liquid_global_entry = cur.read_u16::<LittleEndian>()?;
        self.liquid_off_x = cur.read_u8()?;
        self.liquid_off_y = cur.read_u8()?;
        self.liquid_width = cur.read_u8()?;
        self.liquid_height = cur.read_u8()?;
        self.liquid_level = cur.read_f32::<LittleEndian>()?;

        if flags & MAP_LIQUID_NO_TYPE == 0 {
            let mut entries = vec![0u16; 16 * 16];
            cur.read_u16_into::<LittleEndian>(&mut entries)?;
            self.liquid_entry = Some(entries);

            let mut cell_flags = vec![0u8; 16 * 16];
            cur.read_exact(&mut cell_flags)?;
            self.liquid_flags = Some(cell_flags);
        }

        if flags & MAP_LIQUID_NO_HEIGHT == 0 {
            let samples = self.liquid_width as usize * self.liquid_height as usize;
            let mut heights = vec![0f32; samples];
            cur.read_f32_into::<LittleEndian>(&mut heights)?;
            self.liquid_map = Some(heights);
        }

        Ok(())
    }

    /// Area id at a position (the AreaTable *flag*, not the area id).
    pub fn get_area_flag(&self, x: f32, y: f32) -> u16 {
        let Some(ref areas) = self.area_map else {
            return self.grid_area;
        };

        let (lx, ly) = cell_index_16(x, y);
        areas[lx * 16 + ly]
    }

    /// Liquid type flags at a position, before any DBC refinement.
    pub fn get_terrain_type(&self, x: f32, y: f32) -> u8 {
        let Some(ref cell_flags) = self.liquid_flags else {
            return self.liquid_global_flags;
        };

        let (lx, ly) = cell_index_16(x, y);
        cell_flags[lx * 16 + ly]
    }

    /// Ground height at a position, or `INVALID_HEIGHT_VALUE` if unavailable
    /// (no height data, or the position falls in an ADT hole).
    pub fn get_height(&self, x: f32, y: f32) -> f32 {
        // Fractional sample coordinates within the grid.
        let fx = MAP_RESOLUTION as f32 * (CENTER_GRID_ID as f32 - x / SIZE_OF_GRIDS);
        let fy = MAP_RESOLUTION as f32 * (CENTER_GRID_ID as f32 - y / SIZE_OF_GRIDS);

        let x_int = (fx as i32) & (MAP_RESOLUTION - 1);
        let y_int = (fy as i32) & (MAP_RESOLUTION - 1);
        let fx = fx - fx.floor();
        let fy = fy - fy.floor();

        if self.is_hole(x_int, y_int) {
            return INVALID_HEIGHT_VALUE;
        }

        let (xi, yi) = (x_int as usize, y_int as usize);

        // The height mesh stores four corner samples (V9) plus a centre sample
        // (V8) per square, forming four triangles. Pick the triangle containing
        // (fx, fy), then solve h = a*fx + b*fy + c for its plane.
        //
        //   +--------------> X
        //   | h1-------h2      h1 0,0     1: h1 h2 h5
        //   | | \  1  / |      h2 0,1     2: h1 h3 h5
        //   | |  \   /  |      h3 1,0     3: h2 h4 h5
        //   | | 2  h5 3 |      h4 1,1     4: h3 h4 h5
        //   | |  /   \  |      h5 ½,½
        //   | | /  4  \ |
        //   | h3-------h4
        //   V Y
        macro_rules! solve {
            ($v9:expr, $v8:expr) => {{
                let v9 = |xo: usize, yo: usize| $v9[(xi + xo) * 129 + yi + yo] as f32;
                let h5 = 2.0 * $v8[xi * 128 + yi] as f32;

                let (a, b, c) = if fx + fy < 1.0 {
                    if fx > fy {
                        // triangle 1 (h1, h2, h5)
                        let (h1, h2) = (v9(0, 0), v9(1, 0));
                        (h2 - h1, h5 - h1 - h2, h1)
                    } else {
                        // triangle 2 (h1, h3, h5)
                        let (h1, h3) = (v9(0, 0), v9(0, 1));
                        (h5 - h1 - h3, h3 - h1, h1)
                    }
                } else if fx > fy {
                    // triangle 3 (h2, h4, h5)
                    let (h2, h4) = (v9(1, 0), v9(1, 1));
                    (h2 + h4 - h5, h4 - h2, h5 - h2)
                } else {
                    // triangle 4 (h3, h4, h5)
                    let (h3, h4) = (v9(0, 1), v9(1, 1));
                    (h4 - h3, h3 + h4 - h5, h5 - h4)
                };

                (a * fx) + (b * fy) + c
            }};
        }

        match self.heights {
            HeightMap::Flat => self.grid_height,
            HeightMap::Float { ref v9, ref v8 } => solve!(v9, v8),
            HeightMap::Uint16 { ref v9, ref v8 } => {
                solve!(v9, v8) * self.height_multiplier + self.grid_height
            }
            HeightMap::Uint8 { ref v9, ref v8 } => {
                solve!(v9, v8) * self.height_multiplier + self.grid_height
            }
        }
    }

    /// Liquid surface height at a position, or `INVALID_HEIGHT_VALUE` if the
    /// position lies outside the grid's liquid rectangle.
    pub fn get_liquid_level(&self, x: f32, y: f32) -> f32 {
        let Some(ref liquid_map) = self.liquid_map else {
            return self.liquid_level;
        };

        let (cx, cy) = match self.liquid_sample_index(x, y) {
            Some(idx) => idx,
            None => return INVALID_HEIGHT_VALUE,
        };

        liquid_map[cx * self.liquid_width as usize + cy]
    }

    /// Locate a position within the liquid height rectangle.
    ///
    /// Note the axis swap: the row index is offset by `liquid_off_y` and bounded
    /// by `liquid_height`, matching `GridMap::getLiquidLevel`.
    fn liquid_sample_index(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        let fx = MAP_RESOLUTION as f32 * (CENTER_GRID_ID as f32 - x / SIZE_OF_GRIDS);
        let fy = MAP_RESOLUTION as f32 * (CENTER_GRID_ID as f32 - y / SIZE_OF_GRIDS);

        let cx = ((fx as i32) & (MAP_RESOLUTION - 1)) - self.liquid_off_y as i32;
        let cy = ((fy as i32) & (MAP_RESOLUTION - 1)) - self.liquid_off_x as i32;

        if cx < 0 || cx >= self.liquid_height as i32 {
            return None;
        }
        if cy < 0 || cy >= self.liquid_width as i32 {
            return None;
        }

        Some((cx as usize, cy as usize))
    }

    /// Classify a position against this grid's liquid layer.
    ///
    /// `req_liquid_type` filters by `MAP_LIQUID_TYPE_*`; pass `MAP_ALL_LIQUIDS`
    /// to accept any. On a hit, `data` receives the surface and floor heights.
    pub fn get_liquid_status(
        &self,
        x: f32,
        y: f32,
        z: f32,
        req_liquid_type: u32,
        data: Option<&mut LiquidData>,
    ) -> LiquidStatusFlags {
        // No liquid anywhere in this grid.
        if self.liquid_flags.is_none() && self.liquid_global_flags == 0 {
            return LiquidStatusFlags::NO_WATER;
        }

        let (lx, ly) = cell_index_128(x, y);
        let idx = (lx >> 3) * 16 + (ly >> 3);

        let type_flags = match self.liquid_flags {
            Some(ref f) => f[idx] as u32,
            None => self.liquid_global_flags as u32,
        };
        let entry = match self.liquid_entry {
            Some(ref e) => e[idx] as u32,
            None => self.liquid_global_entry as u32,
        };

        if type_flags == 0 {
            return LiquidStatusFlags::NO_WATER;
        }

        if req_liquid_type != 0 && (req_liquid_type & type_flags) == 0 {
            return LiquidStatusFlags::NO_WATER;
        }

        // Reject positions outside the liquid rectangle, then read the surface.
        let liquid_level = match self.liquid_map {
            Some(ref liquid_map) => match self.liquid_sample_index(x, y) {
                Some((cx, cy)) => liquid_map[cx * self.liquid_width as usize + cy],
                None => return LiquidStatusFlags::NO_WATER,
            },
            None => {
                if self.liquid_sample_index(x, y).is_none() {
                    return LiquidStatusFlags::NO_WATER;
                }
                self.liquid_level
            }
        };

        let ground_level = self.get_height(x, y);

        // Liquid below the floor is stale data; 2 yards of slack lets a player
        // standing in a shallow puddle still register.
        if liquid_level < ground_level || z < ground_level - 2.0 {
            return LiquidStatusFlags::NO_WATER;
        }

        if let Some(data) = data {
            data.entry = entry;
            data.type_flags = type_flags;
            data.level = liquid_level;
            data.depth_level = ground_level;
        }

        classify_depth(liquid_level, z)
    }
}

/// Classify a Z against a liquid surface. Shared by the ADT and WMO paths.
pub(crate) fn classify_depth(liquid_level: f32, z: f32) -> LiquidStatusFlags {
    // Compared as tenths of a yard, matching the reference's integer check.
    let delta = ((liquid_level - z) * 10.0) as i32;

    if delta > 20 {
        LiquidStatusFlags::UNDER_WATER
    } else if delta > 0 {
        LiquidStatusFlags::IN_WATER
    } else if delta > -1 {
        LiquidStatusFlags::WATER_WALK
    } else {
        LiquidStatusFlags::ABOVE_WATER
    }
}

/// Index into a 16×16 per-cell array.
fn cell_index_16(x: f32, y: f32) -> (usize, usize) {
    let fx = 16.0 * (CENTER_GRID_ID as f32 - x / SIZE_OF_GRIDS);
    let fy = 16.0 * (CENTER_GRID_ID as f32 - y / SIZE_OF_GRIDS);
    (((fx as i32) & 15) as usize, ((fy as i32) & 15) as usize)
}

/// Index into a 128×128 per-sample array.
fn cell_index_128(x: f32, y: f32) -> (usize, usize) {
    let fx = MAP_RESOLUTION as f32 * (CENTER_GRID_ID as f32 - x / SIZE_OF_GRIDS);
    let fy = MAP_RESOLUTION as f32 * (CENTER_GRID_ID as f32 - y / SIZE_OF_GRIDS);
    (
        (((fx as i32) & (MAP_RESOLUTION - 1)) as usize),
        (((fy as i32) & (MAP_RESOLUTION - 1)) as usize),
    )
}

impl GridMap {
    /// Whether the height mesh has a hole at the given sample.
    fn is_hole(&self, row: i32, col: i32) -> bool {
        let Some(ref holes) = self.holes else {
            return false;
        };

        let cell_row = (row / 8) as usize;
        let cell_col = (col / 8) as usize;
        let hole_row = ((row % 8) / 2) as usize;
        let hole_col = ((col - (cell_col as i32 * 8)) / 2) as usize;

        if cell_row >= 16 || cell_col >= 16 || hole_row >= 4 || hole_col >= 4 {
            return false;
        }

        let hole = holes[cell_row * 16 + cell_col];
        (hole & HOLETAB_H[hole_col] & HOLETAB_V[hole_row]) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_depth_matches_reference_thresholds() {
        // Surface 2.5 yards above the position -> fully submerged.
        assert_eq!(classify_depth(10.0, 7.5), LiquidStatusFlags::UNDER_WATER);
        // Surface 1 yard above -> in water but head is out.
        assert_eq!(classify_depth(10.0, 9.0), LiquidStatusFlags::IN_WATER);
        // Standing exactly at the surface -> water walk.
        assert_eq!(classify_depth(10.0, 10.0), LiquidStatusFlags::WATER_WALK);
        // Clearly above the surface.
        assert_eq!(classify_depth(10.0, 12.0), LiquidStatusFlags::ABOVE_WATER);
    }

    #[test]
    fn status_flags_mask_membership() {
        assert!(LiquidStatusFlags::UNDER_WATER.intersects(LiquidStatusFlags::MASK_SWIMMING));
        assert!(LiquidStatusFlags::IN_WATER.intersects(LiquidStatusFlags::MASK_SWIMMING));
        assert!(!LiquidStatusFlags::WATER_WALK.intersects(LiquidStatusFlags::MASK_SWIMMING));
        assert!(LiquidStatusFlags::WATER_WALK.intersects(LiquidStatusFlags::MASK_TOUCHING));
        assert!(LiquidStatusFlags::NO_WATER.is_empty());
    }

    #[test]
    fn terrain_grid_coords_map_origin_to_centre() {
        assert_eq!(terrain_grid_coords(0.0, 0.0), Some((32, 32)));
    }

    #[test]
    fn terrain_grid_coords_swap_and_mirror_axes() {
        // gx derives from y, gy from x — the `.map` filename convention.
        let (gx, gy) = terrain_grid_coords(6400.0, -1600.0).unwrap();
        assert_eq!((gx, gy), (35, 20));
    }

    #[test]
    fn terrain_grid_coords_reject_out_of_bounds() {
        assert_eq!(terrain_grid_coords(60000.0, 0.0), None);
    }

    #[test]
    fn liquid_data_helpers_classify_type_flags() {
        let magma = LiquidData {
            type_flags: MAP_LIQUID_TYPE_MAGMA,
            entry: 0,
            level: 5.0,
            depth_level: 1.0,
        };
        assert!(magma.is_magma());
        assert!(!magma.is_water());
        assert_eq!(magma.depth(), 4.0);

        let ocean = LiquidData {
            type_flags: MAP_LIQUID_TYPE_OCEAN | MAP_LIQUID_TYPE_DEEP_WATER,
            entry: 0,
            level: 0.0,
            depth_level: -500.0,
        };
        assert!(ocean.is_water());
        assert!(ocean.is_deep_water());
        assert!(!ocean.is_slime());
    }

    #[test]
    fn liquid_data_depth_clamps_at_zero() {
        let inverted = LiquidData {
            type_flags: MAP_LIQUID_TYPE_WATER,
            entry: 0,
            level: 1.0,
            depth_level: 5.0,
        };
        assert_eq!(inverted.depth(), 0.0);
    }
}
