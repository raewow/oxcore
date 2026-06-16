use crate::file_loader::DbcRecord;
use crate::store::DbcEntry;
use anyhow::{Context, Result};
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


