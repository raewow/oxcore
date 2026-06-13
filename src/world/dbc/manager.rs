use crate::world::dbc::store::{load_dbc_store, DbcStore};
use crate::world::dbc::structures::{
    AreaTableEntry, AreaTriggerEntry, AuctionHouseEntry, BankBagSlotPricesEntry, ChrClassesEntry,
    ChrRacesEntry, CreatureDisplayInfoEntry, FactionDbcEntry, FactionTemplateDbcEntry,
    GameObjectDisplayInfoEntry, ItemEntry, LockEntry, MapEntry, SkillLineEntry,
    SkillRaceClassInfoEntry, SkillTiersEntry, SpellCastTimeEntry, SpellDurationEntry,
    SpellFocusObjectEntry, SpellRadiusEntry, SpellRangeEntry, TalentEntry, TalentTabEntry,
    WorldSafeLocsEntry,
};
use anyhow::{Context, Result};
use std::path::Path;
use tracing::{debug, error, info, warn};

pub struct DbcManager {
    pub area_table: DbcStore<AreaTableEntry>,
    pub area_trigger: DbcStore<AreaTriggerEntry>,
    pub auction_house: DbcStore<AuctionHouseEntry>,
    pub bank_bag_slot_prices: DbcStore<BankBagSlotPricesEntry>,
    pub chr_classes: DbcStore<ChrClassesEntry>,
    pub chr_races: DbcStore<ChrRacesEntry>,
    pub creature_display_info: DbcStore<CreatureDisplayInfoEntry>,
    pub faction: DbcStore<FactionDbcEntry>,
    pub faction_template: DbcStore<FactionTemplateDbcEntry>,
    pub gameobject_display_info: DbcStore<GameObjectDisplayInfoEntry>,
    pub lock: DbcStore<LockEntry>,
    pub map: DbcStore<MapEntry>,
    pub spell_cast_time: DbcStore<SpellCastTimeEntry>,
    pub spell_duration: DbcStore<SpellDurationEntry>,
    pub spell_focus_object: DbcStore<SpellFocusObjectEntry>,
    pub spell_radius: DbcStore<SpellRadiusEntry>,
    pub spell_range: DbcStore<SpellRangeEntry>,
    pub item: DbcStore<ItemEntry>,
    pub skill_line: DbcStore<SkillLineEntry>,
    pub skill_tiers: DbcStore<SkillTiersEntry>,
    pub skill_race_class_info: DbcStore<SkillRaceClassInfoEntry>,
    pub talent: DbcStore<TalentEntry>,
    pub talent_tab: DbcStore<TalentTabEntry>,
    pub world_safe_locs: DbcStore<WorldSafeLocsEntry>,
}

impl DbcManager {
    pub fn new() -> Self {
        // Faction.dbc format: "iii" + 88*i (for BaseRepValue array) + other fields
        // For vanilla 1.12.1, format string:
        // i = int32/u32 (ID), i = int32 (reputationListID), then 88*i for BaseRepValue array
        // Note: 'n' is not a valid format char, use 'i' for uint32/int32 fields
        // The format string needs to match at least the fields we care about (first 90 fields)
        // If the DBC has more fields, they'll be ignored (treated as 'x')
        let faction_format = "iii".to_string() + &"i".repeat(88);
        // ChrRaces.dbc format: "n" + many fields
        // We need: field 0 (id), field 4 (model_m), field 5 (model_f), field 12 (resSicknessSpellId), field 16 (cinematicSequenceId)
        // Format: "n" (id) + "x"*4 (skip 1-3) + "i" (field 4) + "i" (field 5) + "x"*6 (skip 6-11) + "i" (field 12) + "x"*3 (skip 13-15) + "i" (field 16) + "x"*many
        let chr_races_format = "n".to_string()
            + &"x".repeat(4)
            + "ii"
            + &"x".repeat(6)
            + "i"
            + &"x".repeat(3)
            + "i"
            + &"x".repeat(20);
        // ChrClasses.dbc format: "n" + many fields
        // Format: "n" (id) + "i"*2 (Flags, PowerType) + "i"*3 (string offsets - treated as u32) + "i"*5 (SpellFamily, CinematicSequenceID, RequiredExpansion, ArmorType, DisplayPower) + "i" (WeaponProficiencyMask) + "i" (ArmorProficiencyMask)
        // Field indices: 0=ID, 1=Flags, 2=PowerType, 3-5=strings (offsets, read as u32), 6-10=u32s, 11=WeaponMask, 12=ArmorMask
        // String offsets are u32 values, so we read them as "i" (u32) and skip them
        let chr_classes_format =
            "nii".to_string() + &"i".repeat(3) + &"i".repeat(5) + "ii" + &"x".repeat(200);
        // Map.dbc format: "n" + many fields - we only need fields 0, 2, 4, 5
        // Format string: "n" (id) + "x" (skip field 1) + "i" (mapType) + "x" (skip field 3) + "i" (maxPlayers) + "i" (resetDelay) + "x"*many (skip rest)
        let map_format = "nxiixi".to_string() + &"x".repeat(200); // Read first 6 fields, skip rest

        Self {
            area_table: DbcStore::new("niiixxxxxxxxxxxxxxxxxxxiiixx"),
            area_trigger: DbcStore::new("niffffffff"),
            auction_house: DbcStore::new("niiixxxxxxxxxxxxxx"),
            bank_bag_slot_prices: DbcStore::new("ni"),
            chr_classes: DbcStore::new(&chr_classes_format),
            chr_races: DbcStore::new(&chr_races_format),
            creature_display_info: DbcStore::new("nixifxxxxxxx"),
            faction: DbcStore::new(&faction_format),
            faction_template: DbcStore::new("iiiiiiiiiiiiii"), // 14 fields: all u32 (ID, faction, flags, masks, enemy[4], friend[4])
            gameobject_display_info: DbcStore::new("nsxxxxxxxxxx"),
            lock: DbcStore::new("niiiiiiiiiiiiiiiiiiiiiiiixxxxxxxx"),
            map: DbcStore::new(&map_format),
            spell_cast_time: DbcStore::new("niii"),
            spell_duration: DbcStore::new("niii"),
            spell_focus_object: DbcStore::new("n"),
            spell_radius: DbcStore::new("nff"),
            spell_range: DbcStore::new("nff"),
            item: DbcStore::new("n"), // Item.dbc - we only need the ID field (field 0)
            // SkillLine.dbc format: "n" (id) + "i" (categoryId) + string offsets (8 locales) + string flags + "i" (spellIcon)
            // Format: "n" (id) + "i" (categoryId) + "x"*10 (skip string offsets/flags) + "i" (spellIcon at field 21)
            skill_line: DbcStore::new("nixxxxxxxxxxxxxxxxixx"), // 22 fields: id, categoryId, skip 10, spellIcon, skip rest
            // SkillTiers.dbc format: "n" (id) + 16*u32 (skillValue) + 16*u32 (maxSkillValue) = 33 fields
            skill_tiers: DbcStore::new(&("n".to_string() + &"i".repeat(32))), // 33 fields: id + 32 u32s
            // SkillRaceClassInfo.dbc format: "n" (id) + "i"*6 (skillId, raceMask, classMask, flags, reqLevel, skillTierId)
            skill_race_class_info: DbcStore::new("niiiiii"), // 7 fields: id + 6 u32s
            // WorldSafeLocs.dbc format: "n" (id) + "i" (mapId) + "fff" (x, y, z) + string offsets
            // Format: "nifff" + skip rest (name strings)
            world_safe_locs: DbcStore::new("nifffxxxxxxxx"), // 13 fields: id, mapId, x, y, z, name[8]
            // Talent.dbc format: "niiiii" + 5 rank_spell_ids + "ii" + skip + "ii" for second prereq
            // Fields: id, tab_id, row, column, rank_spell_ids[5], depends_on_talent, depends_on_rank, skip[6], depends_on_talent_2, depends_on_rank_2
            talent: DbcStore::new("niiiiiiiiiiixxxxxxii"), // 20 fields total
            // TalentTab.dbc format: "n" + name[8] + spell_icon + class_mask + tab_page + name[8] for other locales
            // Fields: id, name[8], spell_icon, class_mask, tab_page, ...
            talent_tab: DbcStore::new("nxxxxxxxxiiixxxxxxxxxxxxxxxxxxxxxxxxxxx"), // 39 fields total
        }
    }

    pub fn load_all(&mut self, dbc_path: &str) -> Result<()> {
        let dbc_path = Path::new(dbc_path);

        if !dbc_path.exists() {
            anyhow::bail!("DBC directory does not exist: {}", dbc_path.display());
        }

        if !dbc_path.is_dir() {
            anyhow::bail!("DBC path is not a directory: {}", dbc_path.display());
        }

        debug!("Loading DBC files from: {}", dbc_path.display());

        let area_table_path = dbc_path.join("AreaTable.dbc");
        if area_table_path.exists() {
            load_dbc_store(&mut self.area_table, area_table_path.to_str().unwrap())
                .context("Failed to load AreaTable.dbc")?;
            debug!("Loaded {} AreaTable entries", self.area_table.len());
        } else {
            warn!("AreaTable.dbc not found, skipping");
        }

        let area_trigger_path = dbc_path.join("AreaTrigger.dbc");
        if area_trigger_path.exists() {
            load_dbc_store(&mut self.area_trigger, area_trigger_path.to_str().unwrap())
                .context("Failed to load AreaTrigger.dbc")?;
            debug!("Loaded {} AreaTrigger entries", self.area_trigger.len());
        } else {
            warn!("AreaTrigger.dbc not found, skipping");
        }

        // Load AuctionHouse.dbc
        let auction_house_path = dbc_path.join("AuctionHouse.dbc");
        if auction_house_path.exists() {
            load_dbc_store(
                &mut self.auction_house,
                auction_house_path.to_str().unwrap(),
            )
            .context("Failed to load AuctionHouse.dbc")?;
            debug!("Loaded {} AuctionHouse entries", self.auction_house.len());
        } else {
            warn!("AuctionHouse.dbc not found, skipping");
        }

        // Load BankBagSlotPrices.dbc
        let bank_bag_path = dbc_path.join("BankBagSlotPrices.dbc");
        if bank_bag_path.exists() {
            load_dbc_store(
                &mut self.bank_bag_slot_prices,
                bank_bag_path.to_str().unwrap(),
            )
            .context("Failed to load BankBagSlotPrices.dbc")?;
            debug!(
                "Loaded {} BankBagSlotPrices entries",
                self.bank_bag_slot_prices.len()
            );
        } else {
            warn!("BankBagSlotPrices.dbc not found, skipping");
        }

        // Load ChrClasses.dbc
        let chr_classes_path = dbc_path.join("ChrClasses.dbc");
        if chr_classes_path.exists() {
            load_dbc_store(&mut self.chr_classes, chr_classes_path.to_str().unwrap())
                .context("Failed to load ChrClasses.dbc")?;
            debug!("Loaded {} ChrClasses entries", self.chr_classes.len());
        } else {
            warn!("ChrClasses.dbc not found, skipping");
        }

        // Load ChrRaces.dbc
        let chr_races_path = dbc_path.join("ChrRaces.dbc");
        if chr_races_path.exists() {
            load_dbc_store(&mut self.chr_races, chr_races_path.to_str().unwrap())
                .context("Failed to load ChrRaces.dbc")?;
            debug!("Loaded {} ChrRaces entries", self.chr_races.len());
        } else {
            warn!("ChrRaces.dbc not found, skipping");
        }

        // Load Map.dbc
        let map_path = dbc_path.join("Map.dbc");
        if map_path.exists() {
            load_dbc_store(&mut self.map, map_path.to_str().unwrap())
                .context("Failed to load Map.dbc")?;
            debug!("Loaded {} Map entries", self.map.len());
        } else {
            warn!("Map.dbc not found, skipping");
        }

        // Load Faction.dbc
        let faction_path = dbc_path.join("Faction.dbc");
        if faction_path.exists() {
            match load_dbc_store(&mut self.faction, faction_path.to_str().unwrap()) {
                Ok(_) => {
                    debug!("Loaded {} Faction entries", self.faction.len());
                    if self.faction.len() == 0 {
                        warn!(
                            "WARNING: Faction.dbc loaded but contains 0 entries - this is unusual!"
                        );
                    }
                }
                Err(e) => {
                    error!("CRITICAL: Failed to load Faction.dbc: {:#}", e);
                    // Try to get more details about the error
                    if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                        error!("IO Error details: {}", io_err);
                    }
                    // Log the full error chain
                    let mut current_err: &dyn std::error::Error = e.as_ref();
                    let mut depth = 0;
                    while depth < 5 {
                        error!("Error chain [{}]: {}", depth, current_err);
                        if let Some(source) = current_err.source() {
                            current_err = source;
                            depth += 1;
                        } else {
                            break;
                        }
                    }
                    // Don't fail completely, but log the error
                    return Err(e).context(
                        "Failed to load Faction.dbc - this is critical for reputation system",
                    );
                }
            }
        } else {
            error!(
                "CRITICAL: Faction.dbc not found at: {}",
                faction_path.display()
            );
            error!(
                "Please ensure Faction.dbc is in the DBC directory: {}",
                dbc_path.display()
            );
        }

        // Load FactionTemplate.dbc
        let faction_template_path = dbc_path.join("FactionTemplate.dbc");
        if faction_template_path.exists() {
            load_dbc_store(
                &mut self.faction_template,
                faction_template_path.to_str().unwrap(),
            )
            .context("Failed to load FactionTemplate.dbc")?;
            debug!(
                "Loaded {} FactionTemplate entries",
                self.faction_template.len()
            );
        } else {
            warn!("FactionTemplate.dbc not found, skipping");
        }

        // Load CreatureDisplayInfo.dbc
        let creature_display_info_path = dbc_path.join("CreatureDisplayInfo.dbc");
        if creature_display_info_path.exists() {
            load_dbc_store(
                &mut self.creature_display_info,
                creature_display_info_path.to_str().unwrap(),
            )
            .context("Failed to load CreatureDisplayInfo.dbc")?;
            debug!(
                "Loaded {} CreatureDisplayInfo entries",
                self.creature_display_info.len()
            );
        } else {
            warn!("CreatureDisplayInfo.dbc not found, skipping (display ID validation will be limited)");
        }

        // Load Item.dbc
        // Note: Item.dbc may not exist in all DBC sets, but items can still be loaded from database
        let item_path = dbc_path.join("Item.dbc");
        if item_path.exists() {
            load_dbc_store(&mut self.item, item_path.to_str().unwrap())
                .context("Failed to load Item.dbc")?;
            debug!("Loaded {} Item entries from DBC", self.item.len());
        } else {
            debug!("Item.dbc not found in DBC directory (items will be loaded from database only)");
        }

        // Load SkillLine.dbc
        let skill_line_path = dbc_path.join("SkillLine.dbc");
        if skill_line_path.exists() {
            load_dbc_store(&mut self.skill_line, skill_line_path.to_str().unwrap())
                .context("Failed to load SkillLine.dbc")?;
            debug!("Loaded {} SkillLine entries", self.skill_line.len());
        } else {
            warn!("SkillLine.dbc not found, skill validation will be limited");
        }

        // Load SkillTiers.dbc
        let skill_tiers_path = dbc_path.join("SkillTiers.dbc");
        if skill_tiers_path.exists() {
            load_dbc_store(&mut self.skill_tiers, skill_tiers_path.to_str().unwrap())
                .context("Failed to load SkillTiers.dbc")?;
            debug!("Loaded {} SkillTiers entries", self.skill_tiers.len());
        } else {
            warn!("SkillTiers.dbc not found, skill step calculation will be limited");
        }

        // Load SkillRaceClassInfo.dbc
        let skill_race_class_info_path = dbc_path.join("SkillRaceClassInfo.dbc");
        if skill_race_class_info_path.exists() {
            load_dbc_store(
                &mut self.skill_race_class_info,
                skill_race_class_info_path.to_str().unwrap(),
            )
            .context("Failed to load SkillRaceClassInfo.dbc")?;
            debug!(
                "Loaded {} SkillRaceClassInfo entries",
                self.skill_race_class_info.len()
            );
        } else {
            warn!("SkillRaceClassInfo.dbc not found, skill race/class validation will be limited");
        }

        // Load SpellCastTimes.dbc
        let spell_cast_time_path = dbc_path.join("SpellCastTimes.dbc");
        if spell_cast_time_path.exists() {
            load_dbc_store(
                &mut self.spell_cast_time,
                spell_cast_time_path.to_str().unwrap(),
            )
            .context("Failed to load SpellCastTimes.dbc")?;
            debug!(
                "Loaded {} SpellCastTimes entries",
                self.spell_cast_time.len()
            );
        } else {
            warn!("SpellCastTimes.dbc not found, spell cast times will not work correctly!");
        }

        // Load SpellDuration.dbc
        let spell_duration_path = dbc_path.join("SpellDuration.dbc");
        if spell_duration_path.exists() {
            load_dbc_store(
                &mut self.spell_duration,
                spell_duration_path.to_str().unwrap(),
            )
            .context("Failed to load SpellDuration.dbc")?;
            debug!("Loaded {} SpellDuration entries", self.spell_duration.len());
        } else {
            warn!("SpellDuration.dbc not found, spell durations will be limited");
        }

        // Load SpellRadius.dbc
        let spell_radius_path = dbc_path.join("SpellRadius.dbc");
        if spell_radius_path.exists() {
            load_dbc_store(&mut self.spell_radius, spell_radius_path.to_str().unwrap())
                .context("Failed to load SpellRadius.dbc")?;
            debug!("Loaded {} SpellRadius entries", self.spell_radius.len());
        } else {
            warn!("SpellRadius.dbc not found, spell radius will be limited");
        }

        // Load SpellRange.dbc
        let spell_range_path = dbc_path.join("SpellRange.dbc");
        if spell_range_path.exists() {
            load_dbc_store(&mut self.spell_range, spell_range_path.to_str().unwrap())
                .context("Failed to load SpellRange.dbc")?;
            debug!("Loaded {} SpellRange entries", self.spell_range.len());
        } else {
            warn!("SpellRange.dbc not found, spell range validation will be limited");
        }

        // Load SpellFocusObject.dbc
        let spell_focus_object_path = dbc_path.join("SpellFocusObject.dbc");
        if spell_focus_object_path.exists() {
            load_dbc_store(
                &mut self.spell_focus_object,
                spell_focus_object_path.to_str().unwrap(),
            )
            .context("Failed to load SpellFocusObject.dbc")?;
            debug!(
                "Loaded {} SpellFocusObject entries",
                self.spell_focus_object.len()
            );
        } else {
            debug!("SpellFocusObject.dbc not found, spell focus validation will be limited");
        }

        // Spells are loaded from SQL (spell_template) instead of Spell.dbc
        // because the DBC field offsets differ from what the code expects.
        // Call load_spells_from_sql() separately after DBC loading.

        // Load WorldSafeLocs.dbc - contains graveyard and safe teleport locations
        let world_safe_locs_path = dbc_path.join("WorldSafeLocs.dbc");
        if world_safe_locs_path.exists() {
            load_dbc_store(
                &mut self.world_safe_locs,
                world_safe_locs_path.to_str().unwrap(),
            )
            .context("Failed to load WorldSafeLocs.dbc")?;
            debug!(
                "Loaded {} WorldSafeLocs entries",
                self.world_safe_locs.len()
            );
        } else {
            warn!("WorldSafeLocs.dbc not found - graveyard teleportation will not work!");
        }

        // Load Talent.dbc - contains talent definitions
        let talent_path = dbc_path.join("Talent.dbc");
        if talent_path.exists() {
            load_dbc_store(&mut self.talent, talent_path.to_str().unwrap())
                .context("Failed to load Talent.dbc")?;
            debug!("Loaded {} Talent entries", self.talent.len());
        } else {
            warn!("Talent.dbc not found - talent system will not work!");
        }

        // Load TalentTab.dbc - contains talent tab (tree) definitions
        let talent_tab_path = dbc_path.join("TalentTab.dbc");
        if talent_tab_path.exists() {
            load_dbc_store(&mut self.talent_tab, talent_tab_path.to_str().unwrap())
                .context("Failed to load TalentTab.dbc")?;
            debug!("Loaded {} TalentTab entries", self.talent_tab.len());
        } else {
            warn!("TalentTab.dbc not found - talent system will not work!");
        }

        info!("DBC loading complete");
        Ok(())
    }

    /// Get area table entry by ID
    pub fn get_area(&self, area_id: u32) -> Option<&AreaTableEntry> {
        self.area_table.lookup(area_id)
    }

    /// Get area trigger entry by ID
    pub fn get_area_trigger(&self, trigger_id: u32) -> Option<&AreaTriggerEntry> {
        self.area_trigger.lookup(trigger_id)
    }

    /// Get all area trigger entries
    pub fn get_all_area_triggers(&self) -> impl Iterator<Item = (&u32, &AreaTriggerEntry)> {
        self.area_trigger.entries()
    }

    /// Get auction house entry by ID
    pub fn get_auction_house(&self, house_id: u32) -> Option<&AuctionHouseEntry> {
        self.auction_house.lookup(house_id)
    }

    /// Iterate all auction house DBC entries.
    pub fn get_all_auction_houses(&self) -> impl Iterator<Item = (&u32, &AuctionHouseEntry)> {
        self.auction_house.entries()
    }

    /// Get bank bag slot price by ID
    pub fn get_bank_bag_price(&self, id: u32) -> Option<&BankBagSlotPricesEntry> {
        self.bank_bag_slot_prices.lookup(id)
    }

    /// Get character class entry by ID
    pub fn get_chr_class(&self, class_id: u32) -> Option<&ChrClassesEntry> {
        self.chr_classes.lookup(class_id)
    }

    /// Get character race entry by ID
    pub fn get_chr_race(&self, race_id: u32) -> Option<&ChrRacesEntry> {
        self.chr_races.lookup(race_id)
    }

    /// Get faction entry by ID
    pub fn get_faction(&self, faction_id: u32) -> Option<&FactionDbcEntry> {
        self.faction.lookup(faction_id)
    }

    /// Get faction template entry by ID
    pub fn get_faction_template(&self, template_id: u32) -> Option<&FactionTemplateDbcEntry> {
        self.faction_template.lookup(template_id)
    }

    /// Get all faction template entries
    pub fn get_all_faction_templates(
        &self,
    ) -> impl Iterator<Item = (&u32, &FactionTemplateDbcEntry)> {
        self.faction_template.entries()
    }

    /// Get map entry by ID
    pub fn get_map(&self, map_id: u32) -> Option<&MapEntry> {
        self.map.lookup(map_id)
    }

    /// Get all faction entries
    pub fn get_all_factions(&self) -> impl Iterator<Item = (&u32, &FactionDbcEntry)> {
        self.faction.entries()
    }

    /// Get spell cast time entry by ID
    pub fn get_spell_cast_time(&self, cast_time_id: u32) -> Option<&SpellCastTimeEntry> {
        self.spell_cast_time.lookup(cast_time_id)
    }

    /// Get spell duration entry by ID
    pub fn get_spell_duration(&self, duration_id: u32) -> Option<&SpellDurationEntry> {
        self.spell_duration.lookup(duration_id)
    }

    /// Get spell radius entry by ID
    pub fn get_spell_radius(&self, radius_id: u32) -> Option<&SpellRadiusEntry> {
        self.spell_radius.lookup(radius_id)
    }

    /// Get spell range entry by ID
    pub fn get_spell_range(&self, range_id: u32) -> Option<&SpellRangeEntry> {
        self.spell_range.lookup(range_id)
    }

    /// Get creature display info entry by ID
    pub fn get_creature_display_info(&self, display_id: u32) -> Option<&CreatureDisplayInfoEntry> {
        self.creature_display_info.lookup(display_id)
    }

    /// Check if CreatureDisplayInfo.dbc is loaded
    pub fn has_creature_display_info(&self) -> bool {
        !self.creature_display_info.is_empty()
    }

    /// Get gameobject display info entry by ID
    pub fn get_gameobject_display_info(
        &self,
        display_id: u32,
    ) -> Option<&GameObjectDisplayInfoEntry> {
        self.gameobject_display_info.lookup(display_id)
    }

    /// Get lock entry by ID
    pub fn get_lock(&self, lock_id: u32) -> Option<&LockEntry> {
        self.lock.lookup(lock_id)
    }

    /// Get spell focus object entry by ID
    pub fn get_spell_focus_object(&self, focus_id: u32) -> Option<&SpellFocusObjectEntry> {
        self.spell_focus_object.lookup(focus_id)
    }

    /// Get item entry by ID
    pub fn get_item(&self, item_id: u32) -> Option<&ItemEntry> {
        self.item.lookup(item_id)
    }

    /// Get all item entries
    pub fn get_all_items(&self) -> impl Iterator<Item = (&u32, &ItemEntry)> {
        self.item.entries()
    }

    /// Get skill line entry by ID
    pub fn get_skill_line(&self, skill_id: u32) -> Option<&SkillLineEntry> {
        self.skill_line.lookup(skill_id)
    }

    /// Get skill tiers entry by ID
    pub fn get_skill_tiers(&self, tier_id: u32) -> Option<&SkillTiersEntry> {
        self.skill_tiers.lookup(tier_id)
    }

    /// Get skill race class info entry for a skill/race/class combination
    /// Matches C++ GetSkillRaceClassInfo()
    pub fn get_skill_race_class_info(
        &self,
        skill_id: u32,
        race: u8,
        class: u8,
    ) -> Option<&SkillRaceClassInfoEntry> {
        // SkillRaceClassInfo is indexed by skill_id (we use skill_id as the key)
        // We need to iterate through all entries with this skill_id and check race/class masks
        for (_, entry) in self.skill_race_class_info.entries() {
            if entry.skill_id != skill_id {
                continue;
            }

            // Check race mask (if mask is set, race must match)
            if entry.race_mask != 0 {
                let race_bit = 1 << (race - 1);
                if (entry.race_mask & race_bit) == 0 {
                    continue;
                }
            }

            // Check class mask (if mask is set, class must match)
            if entry.class_mask != 0 {
                let class_bit = 1 << (class - 1);
                if (entry.class_mask & class_bit) == 0 {
                    continue;
                }
            }

            // Found matching entry
            return Some(entry);
        }

        None
    }

    /// Get world safe locs entry by ID (graveyard/safe teleport location)
    pub fn get_world_safe_locs(&self, safe_loc_id: u32) -> Option<&WorldSafeLocsEntry> {
        self.world_safe_locs.lookup(safe_loc_id)
    }

    /// Get all world safe locs entries
    pub fn get_all_world_safe_locs(&self) -> impl Iterator<Item = (&u32, &WorldSafeLocsEntry)> {
        self.world_safe_locs.entries()
    }

    /// Get talent entry by ID
    pub fn get_talent(&self, talent_id: u32) -> Option<&TalentEntry> {
        self.talent.lookup(talent_id)
    }

    /// Get all talent entries
    pub fn get_all_talents(&self) -> impl Iterator<Item = (&u32, &TalentEntry)> {
        self.talent.entries()
    }

    /// Get talent tab entry by ID
    pub fn get_talent_tab(&self, tab_id: u32) -> Option<&TalentTabEntry> {
        self.talent_tab.lookup(tab_id)
    }

    /// Get all talent tab entries
    pub fn get_all_talent_tabs(&self) -> impl Iterator<Item = (&u32, &TalentTabEntry)> {
        self.talent_tab.entries()
    }

    // Spells are loaded via SpellManager from SQL, not from DBC.
}

impl Default for DbcManager {
    fn default() -> Self {
        Self::new()
    }
}
