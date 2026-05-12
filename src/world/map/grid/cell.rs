//! Cell - smallest spatial unit (33.33 x 33.33 units)

use smallvec::SmallVec;

use crate::shared::protocol::ObjectGuid;

/// A single cell within a grid
#[derive(Debug, Clone)]
pub struct Cell {
    /// Objects in this cell (SmallVec for cache-friendly small allocations)
    objects: SmallVec<[ObjectGuid; 4]>,
    /// Players in this cell (subset of objects)
    players: SmallVec<[ObjectGuid; 2]>,
}

impl Cell {
    /// Create a new empty cell
    pub fn new() -> Self {
        Self {
            objects: SmallVec::new(),
            players: SmallVec::new(),
        }
    }

    /// Add an object to the cell
    pub fn add_object(&mut self, guid: ObjectGuid) {
        if !self.objects.contains(&guid) {
            self.objects.push(guid);
            if guid.is_player() {
                self.players.push(guid);
            }
        }
    }

    /// Remove an object from the cell
    pub fn remove_object(&mut self, guid: ObjectGuid) {
        self.objects.retain(|g| *g != guid);
        if guid.is_player() {
            self.players.retain(|g| *g != guid);
        }
    }

    /// Get all objects in cell
    pub fn objects(&self) -> &[ObjectGuid] {
        &self.objects
    }

    /// Get all players in cell
    pub fn players(&self) -> &[ObjectGuid] {
        &self.players
    }

    /// Check if cell has any players
    pub fn has_players(&self) -> bool {
        !self.players.is_empty()
    }

    /// Get player count in cell
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// Check if cell is empty
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Number of objects in cell
    pub fn len(&self) -> usize {
        self.objects.len()
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::new()
    }
}
