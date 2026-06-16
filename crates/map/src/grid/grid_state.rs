//! Grid state machine

/// Grid states for lazy loading lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridState {
    /// Grid not loaded, no data in memory
    Invalid,
    /// Currently loading creature spawn data
    Loading,
    /// Active with players nearby - creatures updating and visible
    Active,
    /// No players, waiting for unload timeout
    Idle,
    /// Being removed, unloading creatures
    Removal,
}

impl GridState {
    /// Can this grid load new objects?
    pub fn can_load(&self) -> bool {
        matches!(self, GridState::Active)
    }

    /// Should this grid update creatures?
    pub fn should_update(&self) -> bool {
        matches!(self, GridState::Active)
    }

    /// Is the grid currently loaded (Active or Idle)?
    pub fn is_loaded(&self) -> bool {
        matches!(self, GridState::Active | GridState::Idle)
    }

    /// Does this grid need loading?
    pub fn needs_loading(&self) -> bool {
        matches!(self, GridState::Invalid)
    }

    /// Is the grid currently in the loading state?
    pub fn is_loading(&self) -> bool {
        matches!(self, GridState::Loading)
    }

    /// Can the grid transition to active?
    pub fn can_activate(&self) -> bool {
        matches!(self, GridState::Invalid | GridState::Idle)
    }
}

impl Default for GridState {
    fn default() -> Self {
        GridState::Invalid
    }
}
