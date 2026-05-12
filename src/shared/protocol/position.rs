#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub o: f32,
}

impl Position {
    pub const fn new(x: f32, y: f32, z: f32, o: f32) -> Self {
        Self { x, y, z, o }
    }

    pub const fn xyz(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, o: 0.0 }
    }

    pub fn distance_to(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn distance_2d(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn distance_sq(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn is_within_range(&self, other: &Position, range: f32) -> bool {
        self.distance_sq(other) <= range * range
    }

    pub fn angle_to(&self, other: &Position) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        dy.atan2(dx)
    }

    pub fn relocate(&mut self, x: f32, y: f32, z: f32, o: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
        self.o = o;
    }

    pub fn relocate_xyz(&mut self, x: f32, y: f32, z: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
    }

    /// Validate and normalize orientation. Returns true if orientation was already valid.
    pub fn validate_orientation(&mut self) -> bool {
        if !self.o.is_finite() {
            self.o = 0.0;
            false
        } else {
            true
        }
    }
}
