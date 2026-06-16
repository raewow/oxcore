//! DBC Structure Definitions
//!
//! These structures match the DBC file formats used by World of Warcraft.
//! They are used to parse and store DBC data loaded from files.

pub mod character;
pub mod faction;
pub mod item_object;
pub mod skill;
pub mod spell;
pub mod talent;
pub mod world;

pub use character::{ChrClassesEntry, ChrRacesEntry};
pub use faction::{FactionDbcEntry, FactionTemplateDbcEntry};
pub use item_object::{
    AuctionHouseEntry, BankBagSlotPricesEntry, CreatureDisplayInfoEntry, GameObjectDisplayInfoEntry,
    ItemEntry, LockEntry,
};
pub use skill::{SkillLineEntry, SkillRaceClassInfoEntry, SkillTiersEntry};
pub use spell::{
    SpellCastTimeEntry, SpellDurationEntry, SpellEntry, SpellFocusObjectEntry, SpellRadiusEntry,
    SpellRangeEntry,
};
pub use talent::{TalentEntry, TalentTabEntry};
pub use world::{AreaTableEntry, AreaTriggerEntry, MapEntry, WorldSafeLocsEntry};
