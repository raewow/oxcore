use crate::file_loader::DbcRecord;
use crate::store::DbcEntry;
use anyhow::{Context, Result};
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


