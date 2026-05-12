//! Corpse world object module.
//!
//! A Corpse is a first-class world object spawned at a player's death
//! location. Like GameObjects, it has position + display + flags but no
//! movement block. See `CorpseManager` for storage/update packet building.

pub mod manager;

pub use manager::CorpseManager;
