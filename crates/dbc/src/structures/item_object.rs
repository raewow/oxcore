use crate::file_loader::DbcRecord;
use crate::store::DbcEntry;
use anyhow::{Context, Result};
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
