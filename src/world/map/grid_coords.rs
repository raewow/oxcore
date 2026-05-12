//! Grid coordinate conversion utilities

/// Grid size in world units
pub const GRID_SIZE: f32 = 533.33333;

/// Cell size in world units
pub const CELL_SIZE: f32 = 33.33333;

/// Half map size (64 grids / 2 * GRID_SIZE)
pub const MAP_HALF_SIZE: f32 = 17066.66667;

/// Number of cells per grid side
pub const CELLS_PER_GRID: u8 = 16;

/// Grid pair (x, y)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPair {
    pub x: u8,
    pub y: u8,
}

impl GridPair {
    pub fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }

    pub fn from_world_coords(x: f32, y: f32) -> Self {
        let (gx, gy) = world_to_grid(x, y);
        Self { x: gx, y: gy }
    }
}

/// Cell pair (x, y) - absolute coordinates (0-1023)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellPair {
    pub x: u16,
    pub y: u16,
}

impl CellPair {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }

    pub fn from_world_coords(x: f32, y: f32) -> Self {
        let (cx, cy) = world_to_cell(x, y);
        Self { x: cx, y: cy }
    }

    /// Get the grid this cell belongs to
    pub fn to_grid_pair(&self) -> GridPair {
        GridPair {
            x: (self.x / CELLS_PER_GRID as u16) as u8,
            y: (self.y / CELLS_PER_GRID as u16) as u8,
        }
    }

    /// Get cell coordinates within the grid (0-15)
    pub fn cell_in_grid(&self) -> (u8, u8) {
        (
            (self.x % CELLS_PER_GRID as u16) as u8,
            (self.y % CELLS_PER_GRID as u16) as u8,
        )
    }
}

/// Convert world coordinates to grid coordinates (0-63)
pub fn world_to_grid(x: f32, y: f32) -> (u8, u8) {
    let grid_x = ((x + MAP_HALF_SIZE) / GRID_SIZE).floor() as i32;
    let grid_y = ((y + MAP_HALF_SIZE) / GRID_SIZE).floor() as i32;

    (grid_x.clamp(0, 63) as u8, grid_y.clamp(0, 63) as u8)
}

/// Convert world coordinates to absolute cell coordinates (0-1023)
pub fn world_to_cell(x: f32, y: f32) -> (u16, u16) {
    let cell_x = ((x + MAP_HALF_SIZE) / CELL_SIZE).floor() as i32;
    let cell_y = ((y + MAP_HALF_SIZE) / CELL_SIZE).floor() as i32;

    (cell_x.clamp(0, 1023) as u16, cell_y.clamp(0, 1023) as u16)
}

/// Convert world coordinates to cell coordinates within a specific grid (0-15)
pub fn world_to_cell_in_grid(x: f32, y: f32, _grid_x: u8, _grid_y: u8) -> (u8, u8) {
    let (abs_cell_x, abs_cell_y) = world_to_cell(x, y);
    (
        (abs_cell_x % CELLS_PER_GRID as u16) as u8,
        (abs_cell_y % CELLS_PER_GRID as u16) as u8,
    )
}

/// Convert grid coordinates to world coordinates (center of grid)
pub fn grid_to_world(grid_x: u8, grid_y: u8) -> (f32, f32) {
    let x = (grid_x as f32 * GRID_SIZE) - MAP_HALF_SIZE + (GRID_SIZE / 2.0);
    let y = (grid_y as f32 * GRID_SIZE) - MAP_HALF_SIZE + (GRID_SIZE / 2.0);
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_origin() {
        // Origin (0, 0) should be in grid (32, 32)
        let (gx, gy) = world_to_grid(0.0, 0.0);
        assert_eq!(gx, 32);
        assert_eq!(gy, 32);
    }

    #[test]
    fn test_grid_corners() {
        // Top-left corner
        let (gx, gy) = world_to_grid(-17066.0, -17066.0);
        assert_eq!(gx, 0);
        assert_eq!(gy, 0);

        // Bottom-right corner
        let (gx, gy) = world_to_grid(17066.0, 17066.0);
        assert_eq!(gx, 63);
        assert_eq!(gy, 63);
    }
}
