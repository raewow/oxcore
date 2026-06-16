//! Trainer data types

/// A spell offered by a trainer (from DB)
#[derive(Debug, Clone)]
pub struct TrainerSpell {
    /// The spell ID (from npc_trainer.spell or npc_trainer_template.spell)
    pub spell_id: u32,
    /// Gold cost in copper
    pub cost: u32,
    /// Required skill ID (0 = none)
    pub req_skill: u16,
    /// Required skill value
    pub req_skill_value: u16,
    /// Required player level
    pub req_level: u8,
}

/// Trainer type constants (from creature_template.trainer_type)
pub mod trainer_type {
    pub const CLASS: u8 = 0;
    pub const MOUNT: u8 = 1;
    pub const TRADESKILL: u8 = 2;
    pub const PET: u8 = 3;
}

/// Trainer spell state sent to client
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TrainerSpellState {
    /// Spell is green (can learn)
    Green = 0,
    /// Spell is red (can't learn - requirements not met)
    Red = 1,
    /// Spell is grey (already known)
    Gray = 2,
}
