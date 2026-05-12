//! DBC Structure Definitions
//!
//! These structures match the DBC file formats used by World of Warcraft.
//! They are used to parse and store DBC data loaded from files.

use crate::world::dbc::file_loader::DbcRecord;
use crate::world::dbc::store::DbcEntry;
use anyhow::{Context, Result};

/// ChrClasses DBC entry
/// Contains class-specific data including proficiency masks
/// Format: "n" + many fields - we need fields for weapon/armor proficiency masks
/// In vanilla 1.12.1, ChrClasses.dbc has fields:
/// - Field 0: ID (uint32)
/// - Field 1: Flags (uint32)
/// - Field 2: PowerType (uint32)
/// - Field 3: Name (string, offset)
/// - Field 4: NameFemale (string, offset)
/// - Field 5: NameMale (string, offset)
/// - Field 6: SpellFamily (uint32)
/// - Field 7: CinematicSequenceID (uint32)
/// - Field 8: RequiredExpansion (uint32)
/// - Field 9: ArmorType (uint32)
/// - Field 10: DisplayPower (uint32)
/// - Field 11: WeaponProficiencyMask (uint32) - bitmask for weapon proficiencies
/// - Field 12: ArmorProficiencyMask (uint32) - bitmask for armor proficiencies
#[derive(Debug, Clone)]
pub struct ChrClassesEntry {
    pub id: u32,
    pub weapon_proficiency_mask: u32,
    pub armor_proficiency_mask: u32,
}

impl DbcEntry for ChrClassesEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read ChrClasses ID")?;

        // Skip if ID is 0 (invalid entry)
        if id == 0 {
            return Ok(None);
        }

        // ChrClasses.dbc format: "n" (id) + many string offsets + uint32 fields
        // We need to skip string fields (they're offsets, not direct values)
        // Field 11 is WeaponProficiencyMask, Field 12 is ArmorProficiencyMask
        // But we need to account for string offsets in between
        // For vanilla 1.12.1, the format is approximately:
        // "n" (id) + "x"*10 (skip to field 11) + "i" (weapon mask) + "i" (armor mask)

        // Try to read weapon proficiency mask (field 11) and armor proficiency mask (field 12)
        // Note: String fields are stored as offsets, so we need to skip them
        // In vanilla 1.12.1 ChrClasses.dbc, the structure is:
        // ID (u32), Flags (u32), PowerType (u32), Name (string offset), NameFemale (string offset),
        // NameMale (string offset), SpellFamily (u32), CinematicSequenceID (u32),
        // RequiredExpansion (u32), ArmorType (u32), DisplayPower (u32),
        // WeaponProficiencyMask (u32), ArmorProficiencyMask (u32)

        // String offsets are u32 values, so we can read them as u32 and skip
        // Field indices: 0=ID, 1=Flags, 2=PowerType, 3=Name, 4=NameFemale, 5=NameMale,
        // 6=SpellFamily, 7=CinematicSequenceID, 8=RequiredExpansion, 9=ArmorType, 10=DisplayPower,
        // 11=WeaponProficiencyMask, 12=ArmorProficiencyMask

        let weapon_mask = record
            .get_u32(11)
            .context("Failed to read ChrClasses WeaponProficiencyMask")?;
        let armor_mask = record
            .get_u32(12)
            .context("Failed to read ChrClasses ArmorProficiencyMask")?;

        let entry = Self {
            id,
            weapon_proficiency_mask: weapon_mask,
            armor_proficiency_mask: armor_mask,
        };

        Ok(Some((id, entry)))
    }
}

/// AreaTable DBC entry
/// Format: "niiixxxxxxxxxxxxxxxxxxxiiixx" (25 fields)
#[derive(Debug, Clone)]
pub struct AreaTableEntry {
    pub id: u32,
    pub map_id: u32,
    pub zone: u32,
    pub explore_flag: u32,
    pub flags: u32,
    pub area_level: i32,
    pub team: u32,
    pub liquid_type_override: u32,
}

impl DbcEntry for AreaTableEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read AreaTable ID")?;

        // Skip if ID is 0 (invalid entry)
        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            map_id: record
                .get_u32(1)
                .context("Failed to read AreaTable map_id")?,
            zone: record.get_u32(2).context("Failed to read AreaTable zone")?,
            explore_flag: record
                .get_u32(3)
                .context("Failed to read AreaTable explore_flag")?,
            flags: record
                .get_u32(4)
                .context("Failed to read AreaTable flags")?,
            area_level: record
                .get_u32(10)
                .context("Failed to read AreaTable area_level")? as i32,
            team: record
                .get_u32(20)
                .context("Failed to read AreaTable team")?,
            liquid_type_override: record
                .get_u32(24)
                .context("Failed to read AreaTable liquid_type_override")?,
        };

        Ok(Some((id, entry)))
    }
}

/// Map DBC entry
/// Format: "niiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii.dbc format is complex - let me add a simpler MapEntry structure that reads the key fields we need.
#[derive(Debug, Clone)]
pub struct MapEntry {
    pub id: u32,
    pub map_type: u32, // 0=Common, 1=Instance, 2=Raid, 3=Battleground
    pub max_players: u32,
    pub reset_delay: u32, // Reset delay in seconds
}

impl DbcEntry for MapEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read Map ID")?;

        // Note: Map ID 0 is Eastern Kingdoms - it's valid! Don't skip it.
        let entry = Self {
            id,
            map_type: record.get_u32(1).context("Failed to read Map map_type")?,
            max_players: record
                .get_u32(2)
                .context("Failed to read Map max_players")?,
            reset_delay: record
                .get_u32(3)
                .context("Failed to read Map reset_delay")?,
        };

        Ok(Some((id, entry)))
    }
}

/// AreaTrigger DBC entry
#[derive(Debug, Clone)]
pub struct AreaTriggerEntry {
    pub id: u32,
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub radius: f32,
    pub box_x: f32,
    pub box_y: f32,
    pub box_z: f32,
    pub box_orientation: f32,
}

impl DbcEntry for AreaTriggerEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read AreaTrigger ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            map_id: record
                .get_u32(1)
                .context("Failed to read AreaTrigger map_id")?,
            x: record.get_f32(2).context("Failed to read AreaTrigger x")?,
            y: record.get_f32(3).context("Failed to read AreaTrigger y")?,
            z: record.get_f32(4).context("Failed to read AreaTrigger z")?,
            radius: record
                .get_f32(5)
                .context("Failed to read AreaTrigger radius")?,
            box_x: record
                .get_f32(6)
                .context("Failed to read AreaTrigger box_x")?,
            box_y: record
                .get_f32(7)
                .context("Failed to read AreaTrigger box_y")?,
            box_z: record
                .get_f32(8)
                .context("Failed to read AreaTrigger box_z")?,
            box_orientation: record
                .get_f32(9)
                .context("Failed to read AreaTrigger box_orientation")?,
        };

        Ok(Some((id, entry)))
    }
}

/// AuctionHouse DBC entry
/// Format: "niiixxxxxxxxxxxxxx" (20 fields)
#[derive(Debug, Clone)]
pub struct AuctionHouseEntry {
    pub house_id: u32,
    pub faction: u32,
    pub deposit_percent: u32,
    pub cut_percent: u32,
}

impl DbcEntry for AuctionHouseEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let house_id = record
            .get_u32(0)
            .context("Failed to read AuctionHouse house_id")?;

        if house_id == 0 {
            return Ok(None);
        }

        let entry = Self {
            house_id,
            faction: record
                .get_u32(1)
                .context("Failed to read AuctionHouse faction")?,
            deposit_percent: record
                .get_u32(2)
                .context("Failed to read AuctionHouse deposit_percent")?,
            cut_percent: record
                .get_u32(3)
                .context("Failed to read AuctionHouse cut_percent")?,
        };

        Ok(Some((house_id, entry)))
    }
}

/// BankBagSlotPrices DBC entry
/// Format: "ni" (2 fields)
#[derive(Debug, Clone)]
pub struct BankBagSlotPricesEntry {
    pub id: u32,
    pub price: u32,
}

impl DbcEntry for BankBagSlotPricesEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read BankBagSlotPrices ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            price: record
                .get_u32(1)
                .context("Failed to read BankBagSlotPrices price")?,
        };

        Ok(Some((id, entry)))
    }
}

/// Faction DBC entry
/// Format: "nii" + 88 BaseRepValue fields (i) + other fields
/// Faction.dbc structure: ID, reputationListID, BaseRepValue[88] (8 races * 11 classes), and other fields
/// For vanilla 1.12.1, the format string is approximately: "nii" + 88*i + other fields
/// Simplified format: "nii" + 88*i = "nii" + 88*i = "nii" + 88*i
/// Faction DBC entry structure (matches C++ FactionEntry)
/// Format: ID, reputationListID, BaseRepRaceMask[4], BaseRepClassMask[4], BaseRepValue[4], ReputationFlags[4], team
/// Note: The DBC may also have an 88-value array format, but we use the mask-based format to match C++ core
#[derive(Debug, Clone)]
pub struct FactionDbcEntry {
    pub id: u32,
    pub reputation_list_id: i32,
    /// Base reputation race masks (4 entries)
    pub base_rep_race_mask: [u32; 4],
    /// Base reputation class masks (4 entries)
    pub base_rep_class_mask: [u32; 4],
    /// Base reputation values (4 entries, one per mask combination)
    pub base_rep_value: [i32; 4],
    /// Reputation flags (4 entries, determines visibility and war state)
    pub reputation_flags: [u32; 4],
    /// Team ID (parent faction)
    pub team: u32,
    /// Legacy: Base reputation values for each race/class combination (88 values)
    /// This is kept for backward compatibility but the mask-based system is preferred
    pub base_rep_value_legacy: [i32; 88],
}

impl FactionDbcEntry {
    /// Get the base reputation for a specific race/class combination
    ///
    /// This looks up the base reputation value by matching the race and class
    /// against the mask arrays in the DBC entry.
    pub fn get_base_reputation(&self, race: u8, class: u8) -> i32 {
        let race_mask = 1u32 << (race - 1);
        let class_mask = 1u32 << (class - 1);

        // Find the matching mask combination
        for i in 0..4 {
            if (self.base_rep_race_mask[i] & race_mask) != 0
                && (self.base_rep_class_mask[i] & class_mask) != 0
            {
                return self.base_rep_value[i];
            }
        }

        // Default to 0 if no mask matches
        0
    }
}

impl DbcEntry for FactionDbcEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read Faction ID")?;

        if id == 0 {
            return Ok(None);
        }

        let reputation_list_id = record
            .get_u32(1)
            .context("Failed to read Faction reputationListID")?
            as i32;

        let field_count = record.field_count() as usize;

        // Read mask-based format (matches C++ core structure):
        // Fields 2-5: BaseRepRaceMask[4]
        // Fields 6-9: BaseRepClassMask[4]
        // Fields 10-13: BaseRepValue[4]
        // Fields 14-17: ReputationFlags[4]
        // Field 18: team

        let mut base_rep_race_mask = [0u32; 4];
        let mut base_rep_class_mask = [0u32; 4];
        let mut base_rep_value = [0i32; 4];
        let mut reputation_flags = [0u32; 4];
        let mut team = 0u32;

        // Read masks and values if available
        if field_count > 18 {
            // Read BaseRepRaceMask[4] (fields 2-5)
            for i in 0..4 {
                base_rep_race_mask[i] = record.get_u32(2 + i).unwrap_or(0);
            }
            // Read BaseRepClassMask[4] (fields 6-9)
            for i in 0..4 {
                base_rep_class_mask[i] = record.get_u32(6 + i).unwrap_or(0);
            }
            // Read BaseRepValue[4] (fields 10-13)
            for i in 0..4 {
                base_rep_value[i] = record.get_u32(10 + i).unwrap_or(0) as i32;
            }
            // Read ReputationFlags[4] (fields 14-17)
            for i in 0..4 {
                reputation_flags[i] = record.get_u32(14 + i).unwrap_or(0);
            }
            // Read team (field 18)
            team = record.get_u32(18).unwrap_or(0);
        }

        // Also read legacy 88-value array if present (for backward compatibility)
        let mut base_rep_value_legacy = [0i32; 88];
        if field_count > 2 {
            let available_fields = (field_count - 2).min(88);
            for i in 0..available_fields {
                base_rep_value_legacy[i] = record.get_u32(2 + i).unwrap_or(0) as i32;
            }
        }

        let entry = Self {
            id,
            reputation_list_id,
            base_rep_race_mask,
            base_rep_class_mask,
            base_rep_value,
            reputation_flags,
            team,
            base_rep_value_legacy,
        };

        Ok(Some((id, entry)))
    }
}

/// ChrRaces DBC entry
/// Format: "n" + many fields (ChrRaces.dbc has many fields, we'll read the ones we need)
/// For vanilla 1.12.1, ChrRaces.dbc has approximately 50+ fields
/// We need: ID (field 0), resSicknessSpellId (field 12), CinematicSequence (field 16)
#[derive(Debug, Clone)]
pub struct ChrRacesEntry {
    pub id: u32,
    /// Male model display ID (field 4: m_MaleDisplayId)
    pub model_m: u32,
    /// Female model display ID (field 5: m_FemaleDisplayId)
    pub model_f: u32,
    /// Resurrection sickness spell ID for this race (field 12: m_ResSicknessSpellId)
    pub res_sickness_spell_id: u32,
    /// Cinematic sequence ID for this race (field 16: m_cinematicSequenceId)
    pub cinematic_sequence_id: u32,
}

impl DbcEntry for ChrRacesEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read ChrRaces ID")?;

        if id == 0 {
            return Ok(None);
        }

        // ChrRaces.dbc structure (vanilla 1.12.1):
        // Field 0: RaceID (id)
        // Field 1: Flags
        // Field 2: FactionID
        // Field 3: ExplorationSoundId
        // Field 4: model_m (MaleDisplayId)
        // Field 5: model_f (FemaleDisplayId)
        // Field 12: resSicknessSpellId (m_ResSicknessSpellId)
        // Field 16: CinematicSequence (m_cinematicSequenceId)

        let model_m = record
            .get_u32(4)
            .context("Failed to read ChrRaces model_m")?;
        let model_f = record
            .get_u32(5)
            .context("Failed to read ChrRaces model_f")?;
        let res_sickness_spell_id = record.get_u32(12).unwrap_or(15007); // Default to 15007 if field doesn't exist or is 0
        let cinematic_sequence_id = record.get_u32(16).unwrap_or(0); // Default to 0 if field doesn't exist

        let entry = Self {
            id,
            model_m,
            model_f,
            res_sickness_spell_id,
            cinematic_sequence_id,
        };

        Ok(Some((id, entry)))
    }
}

/// SpellCastTimes DBC entry
/// Format: "niii" (4 fields: ID, CastTime, CastTimePerLevel, MinCastTime)
/// CastTime is in milliseconds
#[derive(Debug, Clone)]
pub struct SpellCastTimeEntry {
    pub id: u32,
    /// Base cast time in milliseconds
    pub cast_time: i32,
    /// Cast time per level in milliseconds (for level-scaling spells)
    pub cast_time_per_level: i32,
    /// Minimum cast time in milliseconds
    pub min_cast_time: i32,
}

impl DbcEntry for SpellCastTimeEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read SpellCastTimes ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            cast_time: record
                .get_i32(1)
                .context("Failed to read SpellCastTimes cast_time")?,
            cast_time_per_level: record
                .get_i32(2)
                .context("Failed to read SpellCastTimes cast_time_per_level")?,
            min_cast_time: record
                .get_i32(3)
                .context("Failed to read SpellCastTimes min_cast_time")?,
        };

        Ok(Some((id, entry)))
    }
}

/// SpellDuration DBC entry
/// Format: "niii" (4 fields: ID, Duration, DurationPerLevel, MaxDuration)
/// Duration values are in milliseconds
#[derive(Debug, Clone)]
pub struct SpellDurationEntry {
    pub id: u32,
    /// Base duration in milliseconds
    pub duration: i32,
    /// Duration per level in milliseconds (for level-scaling spells)
    pub duration_per_level: i32,
    /// Maximum duration in milliseconds
    pub max_duration: i32,
}

impl DbcEntry for SpellDurationEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read SpellDuration ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            duration: record
                .get_u32(1)
                .context("Failed to read SpellDuration duration")? as i32,
            duration_per_level: record
                .get_u32(2)
                .context("Failed to read SpellDuration duration_per_level")?
                as i32,
            max_duration: record
                .get_u32(3)
                .context("Failed to read SpellDuration max_duration")?
                as i32,
        };

        Ok(Some((id, entry)))
    }
}

/// SpellRadius DBC entry
/// Format: "nff" (3 fields: ID, Radius, RadiusPerLevel)
/// Radius values are in yards
#[derive(Debug, Clone)]
pub struct SpellRadiusEntry {
    pub id: u32,
    /// Base radius in yards
    pub radius: f32,
    /// Radius per level in yards (for level-scaling spells)
    pub radius_per_level: f32,
}

impl DbcEntry for SpellRadiusEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read SpellRadius ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            radius: record
                .get_f32(1)
                .context("Failed to read SpellRadius radius")?,
            radius_per_level: record
                .get_f32(2)
                .context("Failed to read SpellRadius radius_per_level")?,
        };

        Ok(Some((id, entry)))
    }
}

/// SpellRange DBC entry
/// Format: "nffff" (5 fields: ID, RangeMin, RangeMinHostile, RangeMax, RangeMaxHostile)
/// Range values are in yards
///
/// Vanilla 1.12.1 SpellRange.dbc layout:
/// Field 0: ID
/// Field 1: RangeMin (friendly)
/// Field 2: RangeMinHostile
/// Field 3: RangeMax (friendly)
/// Field 4: RangeMaxHostile
#[derive(Debug, Clone)]
pub struct SpellRangeEntry {
    pub id: u32,
    /// Minimum range in yards
    pub range_min: f32,
    /// Maximum range in yards
    pub range_max: f32,
}

impl DbcEntry for SpellRangeEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read SpellRange ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            range_min: record
                .get_f32(1)
                .context("Failed to read SpellRange range_min")?,
            range_max: record
                .get_f32(2)
                .context("Failed to read SpellRange range_max")?,
        };

        Ok(Some((id, entry)))
    }
}

/// FactionTemplate DBC entry
/// Format: "niiiiiiiiiiii" (14 fields: ID, faction, factionFlags, ourMask, friendlyMask, hostileMask, enemyFaction[4], friendFaction[4])
/// FactionTemplate.dbc structure matches FactionTemplateEntry from game/faction.rs
#[derive(Debug, Clone)]
pub struct FactionTemplateDbcEntry {
    pub id: u32,
    pub faction: u32,
    pub faction_flags: u32,
    pub our_mask: u32,
    pub friendly_mask: u32,
    pub hostile_mask: u32,
    pub enemy_factions: [u32; 4],
    pub friend_factions: [u32; 4],
}

impl DbcEntry for FactionTemplateDbcEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read FactionTemplate ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            faction: record
                .get_u32(1)
                .context("Failed to read FactionTemplate faction")?,
            faction_flags: record
                .get_u32(2)
                .context("Failed to read FactionTemplate factionFlags")?,
            our_mask: record
                .get_u32(3)
                .context("Failed to read FactionTemplate ourMask")?,
            friendly_mask: record
                .get_u32(4)
                .context("Failed to read FactionTemplate friendlyMask")?,
            hostile_mask: record
                .get_u32(5)
                .context("Failed to read FactionTemplate hostileMask")?,
            enemy_factions: [
                record
                    .get_u32(6)
                    .context("Failed to read FactionTemplate enemyFaction[0]")?,
                record
                    .get_u32(7)
                    .context("Failed to read FactionTemplate enemyFaction[1]")?,
                record
                    .get_u32(8)
                    .context("Failed to read FactionTemplate enemyFaction[2]")?,
                record
                    .get_u32(9)
                    .context("Failed to read FactionTemplate enemyFaction[3]")?,
            ],
            friend_factions: [
                record
                    .get_u32(10)
                    .context("Failed to read FactionTemplate friendFaction[0]")?,
                record
                    .get_u32(11)
                    .context("Failed to read FactionTemplate friendFaction[1]")?,
                record
                    .get_u32(12)
                    .context("Failed to read FactionTemplate friendFaction[2]")?,
                record
                    .get_u32(13)
                    .context("Failed to read FactionTemplate friendFaction[3]")?,
            ],
        };

        Ok(Some((id, entry)))
    }
}

/// CreatureDisplayInfo DBC entry
/// Format: "nixifxxxxxxx" (from DBCfmt.h)
/// Contains display information for creatures
#[derive(Debug, Clone)]
pub struct CreatureDisplayInfoEntry {
    pub id: u32,
    pub model_id: u32,
    pub sound_id: u32,
    pub extended_display_info_id: u32,
    pub creature_model_scale: f32,
}

impl DbcEntry for CreatureDisplayInfoEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read CreatureDisplayInfo ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            model_id: record
                .get_u32(1)
                .context("Failed to read CreatureDisplayInfo modelId")?,
            sound_id: record
                .get_u32(2)
                .context("Failed to read CreatureDisplayInfo soundId")?,
            extended_display_info_id: record
                .get_u32(3)
                .context("Failed to read CreatureDisplayInfo extendedDisplayInfoId")?,
            creature_model_scale: record
                .get_f32(4)
                .context("Failed to read CreatureDisplayInfo creatureModelScale")?,
        };

        Ok(Some((id, entry)))
    }
}

/// Lock DBC entry
/// Format: "niiiiiiiiiiiiiiiiiiiiiiiixxxxxxxx" (from DBCfmt.h)
/// Contains lock information for gameobjects
#[derive(Debug, Clone)]
pub struct LockEntry {
    pub id: u32,
    pub lock_type: [u32; 8],  // Lock types (keys, items, etc.)
    pub lock_index: [u32; 8], // Lock indices
    pub skill: [u32; 8],      // Required skill levels
}

impl DbcEntry for LockEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read Lock ID")?;

        if id == 0 {
            return Ok(None);
        }

        // Lock.dbc has 8 lock types, 8 lock indices, and 8 skill requirements
        // Format: "n" (id) + 8*i (lock_type) + 8*i (lock_index) + 8*i (skill) + 8*x (skip)
        let mut lock_type = [0u32; 8];
        let mut lock_index = [0u32; 8];
        let mut skill = [0u32; 8];

        for i in 0..8 {
            lock_type[i] = record
                .get_u32(1 + i)
                .with_context(|| format!("Failed to read Lock lockType[{}]", i))?;
            lock_index[i] = record
                .get_u32(9 + i)
                .with_context(|| format!("Failed to read Lock lockIndex[{}]", i))?;
            skill[i] = record
                .get_u32(17 + i)
                .with_context(|| format!("Failed to read Lock skill[{}]", i))?;
        }

        let entry = Self {
            id,
            lock_type,
            lock_index,
            skill,
        };

        Ok(Some((id, entry)))
    }
}

/// GameObjectDisplayInfo DBC entry
/// Format: "nsxxxxxxxxxx" (from DBCfmt.h)
/// Contains display information for gameobjects
#[derive(Debug, Clone)]
pub struct GameObjectDisplayInfoEntry {
    pub id: u32,
    pub model_name: String,
}

impl DbcEntry for GameObjectDisplayInfoEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read GameObjectDisplayInfo ID")?;

        if id == 0 {
            return Ok(None);
        }

        let model_name = record
            .get_string(1)
            .context("Failed to read GameObjectDisplayInfo modelName")?
            .to_string();

        let entry = Self { id, model_name };

        Ok(Some((id, entry)))
    }
}

/// SpellFocusObject DBC entry
/// Format: "n" (id only, minimal structure)
/// Contains spell focus object information
#[derive(Debug, Clone)]
pub struct SpellFocusObjectEntry {
    pub id: u32,
}

impl DbcEntry for SpellFocusObjectEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read SpellFocusObject ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self { id };

        Ok(Some((id, entry)))
    }
}

/// Item DBC entry
/// Format: "n" (id only, minimal structure for validation)
/// Contains item information from Item.dbc
/// We only need the ID field for validation purposes
#[derive(Debug, Clone)]
pub struct ItemEntry {
    pub id: u32,
}

impl DbcEntry for ItemEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read Item ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self { id };

        Ok(Some((id, entry)))
    }
}

/// SkillLine DBC entry
/// Format: "n" (id) + "i" (categoryId) + string offsets (8 locales) + "i" (spellIcon)
/// In vanilla 1.12.1, SkillLine.dbc has:
/// - Field 0: ID (uint32)
/// - Field 1: categoryId (int32)
/// - Fields 2-9: name[8] (string offsets)
/// - Field 10: string flags
/// - Field 21: spellIcon (uint32)
#[derive(Debug, Clone)]
pub struct SkillLineEntry {
    pub id: u32,
    pub category_id: i32,
    pub spell_icon: u32,
}

impl DbcEntry for SkillLineEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read SkillLine ID")?;

        if id == 0 {
            return Ok(None);
        }

        // categoryId is stored as int32 in DBC, but DbcRecord only has get_u32
        // We read as u32 and reinterpret as i32 (same as C++ does)
        let category_id_u32 = record
            .get_u32(1)
            .context("Failed to read SkillLine categoryId")?;
        let category_id = category_id_u32 as i32;
        // Skip string fields (2-9 are name offsets, 10 is string flags)
        // Field 21 is spellIcon
        let spell_icon = record
            .get_u32(21)
            .context("Failed to read SkillLine spellIcon")?;

        Ok(Some((
            id,
            Self {
                id,
                category_id,
                spell_icon,
            },
        )))
    }
}

/// SkillTiers DBC entry
/// Format: "n" (id) + 16*u32 (skillValue) + 16*u32 (maxSkillValue) = 33 fields total
/// In vanilla 1.12.1, SkillTiers.dbc has:
/// - Field 0: ID (uint32)
/// - Fields 1-16: skillValue[16] (uint32)
/// - Fields 17-32: maxSkillValue[16] (uint32)
#[derive(Debug, Clone)]
pub struct SkillTiersEntry {
    pub id: u32,
    pub skill_value: [u32; 16],
    pub max_skill_value: [u32; 16],
}

impl DbcEntry for SkillTiersEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read SkillTiers ID")?;

        if id == 0 {
            return Ok(None);
        }

        let mut skill_value = [0u32; 16];
        let mut max_skill_value = [0u32; 16];

        for i in 0..16 {
            skill_value[i] = record
                .get_u32(1 + i)
                .context("Failed to read SkillTiers skillValue")?;
            max_skill_value[i] = record
                .get_u32(17 + i)
                .context("Failed to read SkillTiers maxSkillValue")?;
        }

        Ok(Some((
            id,
            Self {
                id,
                skill_value,
                max_skill_value,
            },
        )))
    }
}

/// SkillRaceClassInfo DBC entry
/// Format: "n" (id) + "i"*6 (skillId, raceMask, classMask, flags, reqLevel, skillTierId)
/// In vanilla 1.12.1, SkillRaceClassInfo.dbc has:
/// - Field 0: ID (uint32) - not used, skillId is the key
/// - Field 1: skillId (uint32)
/// - Field 2: raceMask (uint32)
/// - Field 3: classMask (uint32)
/// - Field 4: flags (uint32)
/// - Field 5: reqLevel (uint32)
/// - Field 6: skillTierId (uint32)
#[derive(Debug, Clone)]
pub struct SkillRaceClassInfoEntry {
    pub id: u32,
    pub skill_id: u32,
    pub race_mask: u32,
    pub class_mask: u32,
    pub flags: u32,
    pub req_level: u32,
    pub skill_tier_id: u32,
}

impl DbcEntry for SkillRaceClassInfoEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read SkillRaceClassInfo ID")?;

        if id == 0 {
            return Ok(None);
        }

        let skill_id = record
            .get_u32(1)
            .context("Failed to read SkillRaceClassInfo skillId")?;
        let race_mask = record
            .get_u32(2)
            .context("Failed to read SkillRaceClassInfo raceMask")?;
        let class_mask = record
            .get_u32(3)
            .context("Failed to read SkillRaceClassInfo classMask")?;
        let flags = record
            .get_u32(4)
            .context("Failed to read SkillRaceClassInfo flags")?;
        let req_level = record
            .get_u32(5)
            .context("Failed to read SkillRaceClassInfo reqLevel")?;
        let skill_tier_id = record
            .get_u32(6)
            .context("Failed to read SkillRaceClassInfo skillTierId")?;

        // Use skill_id as the key for lookup (not the entry ID)
        Ok(Some((
            skill_id,
            Self {
                id,
                skill_id,
                race_mask,
                class_mask,
                flags,
                req_level,
                skill_tier_id,
            },
        )))
    }
}

/// WorldSafeLocs DBC entry
/// Format: "nifffxxxxxxxx" (ID, MapID, X, Y, Z, Name[8])
/// In vanilla 1.12.1, WorldSafeLocs.dbc has:
/// - Field 0: ID (uint32) - safe location ID
/// - Field 1: MapID (uint32) - map ID
/// - Field 2: X (float) - X coordinate
/// - Field 3: Y (float) - Y coordinate
/// - Field 4: Z (float) - Z coordinate
/// - Fields 5-12: Name (8 locale strings - offsets)
///
/// Used for graveyard locations and other safe teleport destinations.
#[derive(Debug, Clone)]
pub struct WorldSafeLocsEntry {
    pub id: u32,
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl DbcEntry for WorldSafeLocsEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read WorldSafeLocs ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            map_id: record
                .get_u32(1)
                .context("Failed to read WorldSafeLocs map_id")?,
            x: record
                .get_f32(2)
                .context("Failed to read WorldSafeLocs x")?,
            y: record
                .get_f32(3)
                .context("Failed to read WorldSafeLocs y")?,
            z: record
                .get_f32(4)
                .context("Failed to read WorldSafeLocs z")?,
        };

        Ok(Some((id, entry)))
    }
}

/// Talent DBC entry
/// Format: "niiiiiiiiiiiiiiiii" (from DBCfmt.h)
/// In vanilla 1.12.1, Talent.dbc has:
/// - Field 0: ID (uint32) - talent ID
/// - Field 1: tab_id (uint32) - which talent tab this belongs to
/// - Field 2: row (uint32) - row position in tree (0-6)
/// - Field 3: column (uint32) - column position in tree (0-3)
/// - Fields 4-8: rank_spell_id[5] (uint32) - spell IDs for ranks 1-5
/// - Field 9: depends_on_talent (uint32) - prerequisite talent ID (0 = none)
/// - Field 10: depends_on_rank (uint32) - required rank of prerequisite
/// - Fields 11-16: additional data (unused in vanilla)
/// - Field 17: depends_on_talent_2 (uint32) - second prerequisite (rare, TBC+)
/// - Field 18: depends_on_rank_2 (uint32) - required rank of second prerequisite
#[derive(Debug, Clone)]
pub struct TalentEntry {
    pub id: u32,
    pub tab_id: u32,
    pub row: u32,
    pub column: u32,
    pub rank_spell_ids: [u32; 5],
    pub prerequisite_talent_id: u32,
    pub prerequisite_rank: u32,
    pub prerequisite_talent_id_2: u32,
    pub prerequisite_rank_2: u32,
}

impl DbcEntry for TalentEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read Talent ID")?;

        if id == 0 {
            return Ok(None);
        }

        let tab_id = record.get_u32(1).context("Failed to read Talent tab_id")?;
        let row = record.get_u32(2).context("Failed to read Talent row")?;
        let column = record.get_u32(3).context("Failed to read Talent column")?;

        let mut rank_spell_ids = [0u32; 5];
        for i in 0..5 {
            rank_spell_ids[i] = record
                .get_u32(4 + i)
                .with_context(|| format!("Failed to read Talent rank_spell_id[{}]", i))?;
        }

        let prerequisite_talent_id = record
            .get_u32(9)
            .context("Failed to read Talent depends_on_talent")?;
        let prerequisite_rank = record
            .get_u32(10)
            .context("Failed to read Talent depends_on_rank")?;

        // Fields 11-16 are additional data, skip to 17-18 for second prerequisite
        let prerequisite_talent_id_2 = record.get_u32(17).unwrap_or(0);
        let prerequisite_rank_2 = record.get_u32(18).unwrap_or(0);

        let entry = Self {
            id,
            tab_id,
            row,
            column,
            rank_spell_ids,
            prerequisite_talent_id,
            prerequisite_rank,
            prerequisite_talent_id_2,
            prerequisite_rank_2,
        };

        Ok(Some((id, entry)))
    }
}

/// TalentTab DBC entry
/// Format: "nixxxxxxxxxxxxxxxxxxiiixxxxxxxxxxxxxxxxxxxxxxxxxxx" (from DBCfmt.h)
/// In vanilla 1.12.1, TalentTab.dbc has:
/// - Field 0: ID (uint32) - tab ID
/// - Fields 1-8: name[8] (string offsets) - localized names
/// - Field 9: spell_icon (uint32) - icon ID
/// - Field 10: class_mask (uint32) - bitmask of classes that have this tab
/// - Field 11: tab_page (uint32) - order within class (0, 1, 2)
/// - Fields 12+: name[8] for other locales (we skip these)
#[derive(Debug, Clone)]
pub struct TalentTabEntry {
    pub id: u32,
    pub spell_icon: u32,
    pub class_mask: u32,
    pub tab_page: u32,
}

impl DbcEntry for TalentTabEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read TalentTab ID")?;

        if id == 0 {
            return Ok(None);
        }

        // Skip name fields (1-8), read spell_icon at field 9
        let spell_icon = record
            .get_u32(9)
            .context("Failed to read TalentTab spell_icon")?;
        let class_mask = record
            .get_u32(10)
            .context("Failed to read TalentTab class_mask")?;
        let tab_page = record
            .get_u32(11)
            .context("Failed to read TalentTab tab_page")?;

        let entry = Self {
            id,
            spell_icon,
            class_mask,
            tab_page,
        };

        Ok(Some((id, entry)))
    }
}

/// Spell DBC entry
/// Format: Complex structure with 176+ fields in vanilla 1.12.1
/// This structure contains all spell data needed for casting and effects
#[derive(Debug, Clone)]
pub struct SpellEntry {
    pub id: u32,
    pub name: String,
    pub school: u32,
    pub category: u32,
    pub dispel: u32,
    pub mechanic: u32,
    pub attributes: u32,
    pub attributes_ex: u32,
    pub attributes_ex2: u32,
    pub attributes_ex3: u32,
    pub attributes_ex4: u32,
    pub stances: u32,
    pub stances_not: u32,
    pub targets: u32,
    pub target_creature_type: u32,
    pub requires_spell_focus: u32,
    pub caster_aura_state: u32,
    pub target_aura_state: u32,
    pub casting_time_index: u32,
    pub recovery_time: u32,
    pub category_recovery_time: u32,
    pub interrupt_flags: u32,
    pub aura_interrupt_flags: u32,
    pub channel_interrupt_flags: u32,
    pub proc_flags: u32,
    pub proc_chance: u32,
    pub proc_charges: u32,
    pub max_level: u32,
    pub base_level: u32,
    pub spell_level: u32,
    pub duration_index: u32,
    pub power_type: u32,
    pub mana_cost: u32,
    pub mana_cost_per_level: u32,
    pub mana_per_second: u32,
    pub mana_per_second_per_level: u32,
    pub range_index: u32,
    pub speed: f32,
    pub stack_amount: u32,
    pub totem: [u32; 2],
    pub reagent: [i32; 8],
    pub reagent_count: [u32; 8],
    pub equipped_item_class: i32,
    pub equipped_item_sub_class_mask: i32,
    pub equipped_item_inventory_type_mask: i32,
    /// Effect types for 3 effects (indices 0-2)
    pub effect: [u32; 3],
    /// Effect die sides for random damage/healing
    pub effect_die_sides: [i32; 3],
    /// Effect base dice
    pub effect_base_dice: [u32; 3],
    /// Effect dice per level
    pub effect_dice_per_level: [f32; 3],
    /// Effect real points per level (scaling)
    pub effect_real_points_per_level: [f32; 3],
    /// Effect base points (main value)
    pub effect_base_points: [i32; 3],
    /// Effect bonus coefficient (spell power scaling)
    pub effect_bonus_coefficient: [f32; 3],
    /// Effect mechanic
    pub effect_mechanic: [u32; 3],
    /// Effect implicit target A
    pub effect_implicit_target_a: [u32; 3],
    /// Effect implicit target B
    pub effect_implicit_target_b: [u32; 3],
    /// Effect radius index
    pub effect_radius_index: [u32; 3],
    /// Effect apply aura name (for aura effects)
    pub effect_apply_aura_name: [u32; 3],
    /// Effect amplitude (periodic tick time)
    pub effect_amplitude: [u32; 3],
    /// Effect multiple value
    pub effect_multiple_value: [f32; 3],
    /// Effect chain target count
    pub effect_chain_target: [u32; 3],
    /// Effect item type
    pub effect_item_type: [u64; 3],
    /// Effect misc value (used by many effects)
    pub effect_misc_value: [i32; 3],
    /// Effect trigger spell
    pub effect_trigger_spell: [u32; 3],
    /// Effect points per combo point
    pub effect_points_per_combo_point: [f32; 3],
    pub spell_visual: u32,
    pub spell_icon_id: u32,
    pub active_icon_id: u32,
    pub spell_priority: u32,
    pub mana_cost_percentage: u32,
    pub start_recovery_category: u32,
    pub start_recovery_time: u32,
    pub max_target_level: u32,
    pub spell_family_name: u32,
    pub spell_family_flags: u64,
    pub max_affected_targets: u32,
    pub dmg_class: u32,
    pub prevention_type: u32,
    pub dmg_multiplier: [f32; 3],
}

impl SpellEntry {
    /// Get the number of non-none effects
    pub fn get_effects_count(&self) -> u8 {
        self.effect.iter().filter(|&&e| e != 0).count() as u8
    }

    /// Check if spell has a specific attribute
    pub fn has_attribute(&self, attribute: u32) -> bool {
        (self.attributes & attribute) != 0
    }

    /// Check if spell has a specific effect
    pub fn has_effect(&self, effect_type: u32) -> bool {
        self.effect.iter().any(|&e| e == effect_type)
    }

    /// Check if spell applies an aura
    pub fn is_aura_spell(&self) -> bool {
        self.effect.iter().enumerate().any(|(i, &e)| {
            e == 6 || e == 27 || e == 35 || e == 119 || e == 128 || e == 129 || e == 132
            // ApplyAura effects
        })
    }
}

impl DbcEntry for SpellEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read Spell ID")?;

        if id == 0 {
            return Ok(None);
        }

        // Spell.dbc has many fields - we'll read the ones we need
        // Format for vanilla 1.12.1 (approximately 176 fields)
        // Fields 0-175 are the main spell data

        let mut effect = [0u32; 3];
        let mut effect_die_sides = [0i32; 3];
        let mut effect_base_dice = [0u32; 3];
        let mut effect_dice_per_level = [0f32; 3];
        let mut effect_real_points_per_level = [0f32; 3];
        let mut effect_base_points = [0i32; 3];
        let mut effect_bonus_coefficient = [0f32; 3];
        let mut effect_mechanic = [0u32; 3];
        let mut effect_implicit_target_a = [0u32; 3];
        let mut effect_implicit_target_b = [0u32; 3];
        let mut effect_radius_index = [0u32; 3];
        let mut effect_apply_aura_name = [0u32; 3];
        let mut effect_amplitude = [0u32; 3];
        let mut effect_multiple_value = [0f32; 3];
        let mut effect_chain_target = [0u32; 3];
        let mut effect_item_type = [0u64; 3];
        let mut effect_misc_value = [0i32; 3];
        let mut effect_trigger_spell = [0u32; 3];
        let mut effect_points_per_combo_point = [0f32; 3];

        // Read effect data (fields 61-117)
        for i in 0..3 {
            effect[i] = record.get_u32(61 + i).unwrap_or(0);
            effect_die_sides[i] = record.get_i32(64 + i).unwrap_or(0);
            effect_base_dice[i] = record.get_u32(67 + i).unwrap_or(0);
            effect_dice_per_level[i] = record.get_f32(70 + i).unwrap_or(0.0);
            effect_real_points_per_level[i] = record.get_f32(73 + i).unwrap_or(0.0);
            effect_base_points[i] = record.get_i32(76 + i).unwrap_or(0);
            effect_bonus_coefficient[i] = record.get_f32(79 + i).unwrap_or(0.0);
            effect_mechanic[i] = record.get_u32(82 + i).unwrap_or(0);
            effect_implicit_target_a[i] = record.get_u32(85 + i).unwrap_or(0);
            effect_implicit_target_b[i] = record.get_u32(88 + i).unwrap_or(0);
            effect_radius_index[i] = record.get_u32(91 + i).unwrap_or(0);
            effect_apply_aura_name[i] = record.get_u32(94 + i).unwrap_or(0);
            effect_amplitude[i] = record.get_u32(97 + i).unwrap_or(0);
            effect_multiple_value[i] = record.get_f32(100 + i).unwrap_or(0.0);
            effect_chain_target[i] = record.get_u32(103 + i).unwrap_or(0);
            // effect_item_type is u64 but DBC only has u32 fields
            // Read as two u32s and combine (low + high << 32)
            let item_type_low = record.get_u32(106 + i).unwrap_or(0);
            let item_type_high = record.get_u32(107 + i).unwrap_or(0);
            effect_item_type[i] = (item_type_low as u64) | ((item_type_high as u64) << 32);
            effect_misc_value[i] = record.get_i32(109 + i).unwrap_or(0);
            effect_trigger_spell[i] = record.get_u32(112 + i).unwrap_or(0);
            effect_points_per_combo_point[i] = record.get_f32(115 + i).unwrap_or(0.0);
        }

        let entry = Self {
            id,
            name: String::new(),
            school: record.get_u32(1).unwrap_or(0),
            category: record.get_u32(2).unwrap_or(0),
            dispel: record.get_u32(4).unwrap_or(0),
            mechanic: record.get_u32(5).unwrap_or(0),
            attributes: record.get_u32(6).unwrap_or(0),
            attributes_ex: record.get_u32(7).unwrap_or(0),
            attributes_ex2: record.get_u32(8).unwrap_or(0),
            attributes_ex3: record.get_u32(9).unwrap_or(0),
            attributes_ex4: record.get_u32(10).unwrap_or(0),
            stances: record.get_u32(11).unwrap_or(0),
            stances_not: record.get_u32(12).unwrap_or(0),
            targets: record.get_u32(13).unwrap_or(0),
            target_creature_type: record.get_u32(14).unwrap_or(0),
            requires_spell_focus: record.get_u32(15).unwrap_or(0),
            caster_aura_state: record.get_u32(16).unwrap_or(0),
            target_aura_state: record.get_u32(17).unwrap_or(0),
            casting_time_index: record.get_u32(18).unwrap_or(0),
            recovery_time: record.get_u32(19).unwrap_or(0),
            category_recovery_time: record.get_u32(20).unwrap_or(0),
            interrupt_flags: record.get_u32(21).unwrap_or(0),
            aura_interrupt_flags: record.get_u32(22).unwrap_or(0),
            channel_interrupt_flags: record.get_u32(23).unwrap_or(0),
            proc_flags: record.get_u32(24).unwrap_or(0),
            proc_chance: record.get_u32(25).unwrap_or(0),
            proc_charges: record.get_u32(26).unwrap_or(0),
            max_level: record.get_u32(27).unwrap_or(0),
            base_level: record.get_u32(28).unwrap_or(0),
            spell_level: record.get_u32(29).unwrap_or(0),
            duration_index: record.get_u32(30).unwrap_or(0),
            power_type: record.get_u32(31).unwrap_or(0),
            mana_cost: record.get_u32(32).unwrap_or(0),
            mana_cost_per_level: record.get_u32(33).unwrap_or(0),
            mana_per_second: record.get_u32(34).unwrap_or(0),
            mana_per_second_per_level: record.get_u32(35).unwrap_or(0),
            range_index: record.get_u32(36).unwrap_or(1),
            speed: record.get_f32(37).unwrap_or(0.0),
            stack_amount: record.get_u32(39).unwrap_or(0),
            totem: [
                record.get_u32(40).unwrap_or(0),
                record.get_u32(41).unwrap_or(0),
            ],
            reagent: [
                record.get_i32(42).unwrap_or(0),
                record.get_i32(43).unwrap_or(0),
                record.get_i32(44).unwrap_or(0),
                record.get_i32(45).unwrap_or(0),
                record.get_i32(46).unwrap_or(0),
                record.get_i32(47).unwrap_or(0),
                record.get_i32(48).unwrap_or(0),
                record.get_i32(49).unwrap_or(0),
            ],
            reagent_count: [
                record.get_u32(50).unwrap_or(0),
                record.get_u32(51).unwrap_or(0),
                record.get_u32(52).unwrap_or(0),
                record.get_u32(53).unwrap_or(0),
                record.get_u32(54).unwrap_or(0),
                record.get_u32(55).unwrap_or(0),
                record.get_u32(56).unwrap_or(0),
                record.get_u32(57).unwrap_or(0),
            ],
            equipped_item_class: record.get_i32(58).unwrap_or(-1),
            equipped_item_sub_class_mask: record.get_i32(59).unwrap_or(0),
            equipped_item_inventory_type_mask: record.get_i32(60).unwrap_or(0),
            effect,
            effect_die_sides,
            effect_base_dice,
            effect_dice_per_level,
            effect_real_points_per_level,
            effect_base_points,
            effect_bonus_coefficient,
            effect_mechanic,
            effect_implicit_target_a,
            effect_implicit_target_b,
            effect_radius_index,
            effect_apply_aura_name,
            effect_amplitude,
            effect_multiple_value,
            effect_chain_target,
            effect_item_type,
            effect_misc_value,
            effect_trigger_spell,
            effect_points_per_combo_point,
            spell_visual: record.get_u32(118).unwrap_or(0),
            spell_icon_id: record.get_u32(120).unwrap_or(0),
            active_icon_id: record.get_u32(121).unwrap_or(0),
            spell_priority: record.get_u32(122).unwrap_or(0),
            mana_cost_percentage: record.get_u32(159).unwrap_or(0),
            start_recovery_category: record.get_u32(160).unwrap_or(0),
            start_recovery_time: record.get_u32(161).unwrap_or(0),
            max_target_level: record.get_u32(163).unwrap_or(0),
            spell_family_name: record.get_u32(164).unwrap_or(0),
            // spell_family_flags is u64 but DBC only has u32 fields
            // Read as two u32s and combine (low + high << 32)
            spell_family_flags: {
                let flags_low = record.get_u32(165).unwrap_or(0);
                let flags_high = record.get_u32(166).unwrap_or(0);
                (flags_low as u64) | ((flags_high as u64) << 32)
            },
            max_affected_targets: record.get_u32(167).unwrap_or(0),
            dmg_class: record.get_u32(168).unwrap_or(0),
            prevention_type: record.get_u32(169).unwrap_or(0),
            dmg_multiplier: [
                record.get_f32(171).unwrap_or(1.0),
                record.get_f32(172).unwrap_or(1.0),
                record.get_f32(173).unwrap_or(1.0),
            ],
        };

        Ok(Some((id, entry)))
    }
}
