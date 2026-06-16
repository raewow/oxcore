pub const TAXI_MASK_SIZE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxiMask(pub [u8; TAXI_MASK_SIZE]);

impl TaxiMask {
    pub fn new() -> Self {
        Self([0u8; TAXI_MASK_SIZE])
    }

    pub fn clear(&mut self) {
        self.0 = [0u8; TAXI_MASK_SIZE];
    }

    pub fn set(&mut self, index: usize) {
        if index < TAXI_MASK_SIZE * 8 {
            self.0[index / 8] |= 1 << (index % 8);
        }
    }

    pub fn unset(&mut self, index: usize) {
        if index < TAXI_MASK_SIZE * 8 {
            self.0[index / 8] &= !(1 << (index % 8));
        }
    }

    pub fn is_set(&self, index: usize) -> bool {
        if index < TAXI_MASK_SIZE * 8 {
            (self.0[index / 8] & (1 << (index % 8))) != 0
        } else {
            false
        }
    }

    pub fn has_any(&self) -> bool {
        self.0.iter().any(|&byte| byte != 0)
    }

    /// Return the mask as an array of u32 values for packet serialization
    pub fn as_array(&self) -> [u32; TAXI_MASK_SIZE] {
        let mut result = [0u32; TAXI_MASK_SIZE];
        for (i, byte) in self.0.iter().enumerate() {
            result[i] = *byte as u32;
        }
        result
    }
}

impl Default for TaxiMask {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaxiNode {
    pub id: u32,
    pub name: String,
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub next_nodes: Vec<u32>,
    pub mount_creature_id: u32,
    pub cost: u32,
}

impl TaxiNode {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: String::new(),
            map_id: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            next_nodes: Vec::new(),
            mount_creature_id: 0,
            cost: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaxiPath {
    pub from_node: u32,
    pub to_node: u32,
    pub id: u32,
    pub cost: u32,
    pub length: f32,
}

impl TaxiPath {
    pub fn new(from_node: u32, to_node: u32) -> Self {
        Self {
            from_node,
            to_node,
            id: 0,
            cost: 0,
            length: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaxiRoute {
    pub path: TaxiPath,
    pub nodes: Vec<TaxiNode>,
    pub total_cost: u32,
    pub total_length: f32,
}

impl TaxiRoute {
    pub fn new(path: TaxiPath) -> Self {
        Self {
            path,
            nodes: Vec::new(),
            total_cost: 0,
            total_length: 0.0,
        }
    }
}
