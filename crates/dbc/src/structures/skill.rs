use crate::file_loader::DbcRecord;
use crate::store::DbcEntry;
use anyhow::{Context, Result};
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


