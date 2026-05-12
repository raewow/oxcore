/// Death state machine for creatures
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DeathState {
    #[default]
    Alive,
    /// Just died this tick - triggers death processing
    JustDied,
    /// Corpse visible, waiting for loot/decay
    Corpse,
    /// Corpse removed, waiting for respawn
    Dead,
}

impl DeathState {
    pub fn is_alive(&self) -> bool {
        matches!(self, DeathState::Alive)
    }

    pub fn is_dead(&self) -> bool {
        !matches!(self, DeathState::Alive)
    }

    pub fn is_corpse(&self) -> bool {
        matches!(self, DeathState::Corpse)
    }
}
