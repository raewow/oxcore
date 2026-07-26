use crate::store::{load_dbc_store, DbcEntry, DbcStore};
use crate::structures::{
    AreaTableEntry, AreaTriggerEntry, AuctionHouseEntry, BankBagSlotPricesEntry, ChrClassesEntry,
    ChrRacesEntry, CreatureDisplayInfoEntry, FactionDbcEntry, FactionTemplateDbcEntry,
    GameObjectDisplayInfoEntry, ItemEntry, LockEntry, MapEntry, SkillLineAbilityEntry,
    SkillLineEntry, SkillRaceClassInfoEntry, SkillTiersEntry, SpellCastTimeEntry,
    SpellDurationEntry, SpellFocusObjectEntry, SpellRadiusEntry, SpellRangeEntry, TalentEntry,
    TalentTabEntry, WorldSafeLocsEntry,
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
    pub skill_line_ability: DbcStore<SkillLineAbilityEntry>,
    pub skill_tiers: DbcStore<SkillTiersEntry>,
    pub skill_race_class_info: DbcStore<SkillRaceClassInfoEntry>,
    pub talent: DbcStore<TalentEntry>,
    pub talent_tab: DbcStore<TalentTabEntry>,
    pub world_safe_locs: DbcStore<WorldSafeLocsEntry>,
}

impl DbcManager {
    pub fn new() -> Self {
        // Computed format strings for DBCs with complex layouts
        let faction_format = "iii".to_string() + &"i".repeat(88);
        let chr_races_format = "n".to_string()
            + &"x".repeat(4)
            + "ii"
            + &"x".repeat(6)
            + "i"
            + &"x".repeat(3)
            + "i"
            + &"x".repeat(20);
        let chr_classes_format =
            "nii".to_string() + &"i".repeat(3) + &"i".repeat(5) + "ii" + &"x".repeat(200);
        let map_format = "nxiixi".to_string() + &"x".repeat(200);

        Self {
            area_table: DbcStore::new("niiixxxxxxxxxxxxxxxxxxxiiixx"),
            area_trigger: DbcStore::new("niffffffff"),
            auction_house: DbcStore::new("niiixxxxxxxxxxxxxx"),
            bank_bag_slot_prices: DbcStore::new("ni"),
            chr_classes: DbcStore::new(&chr_classes_format),
            chr_races: DbcStore::new(&chr_races_format),
            creature_display_info: DbcStore::new("nixifxxxxxxx"),
            faction: DbcStore::new(&faction_format),
            faction_template: DbcStore::new("iiiiiiiiiiiiii"),
            gameobject_display_info: DbcStore::new("nsxxxxxxxxxx"),
            lock: DbcStore::new("niiiiiiiiiiiiiiiiiiiiiiiixxxxxxxx"),
            map: DbcStore::new(&map_format),
            spell_cast_time: DbcStore::new("niii"),
            spell_duration: DbcStore::new("niii"),
            spell_focus_object: DbcStore::new("n"),
            spell_radius: DbcStore::new("nff"),
            // SpellRange.dbc stores min and max range in fields 1 and 2; later
            // fields are flags and localized display names.
            spell_range: DbcStore::new("nffxxxxxxxxxxxxxxxxxxx"),
            item: DbcStore::new("n"),
            skill_line: DbcStore::new("nixxxxxxxxxxxxxxxxixx"),
            skill_line_ability: DbcStore::new("niiiixxiiiiixxi"),
            skill_tiers: DbcStore::new(&("n".to_string() + &"i".repeat(32))),
            skill_race_class_info: DbcStore::new("niiiiii"),
            world_safe_locs: DbcStore::new("nifffxxxxxxxx"),
            talent: DbcStore::new("niiiiiiiiiiixxxxxxii"),
            talent_tab: DbcStore::new("nxxxxxxxxiiixxxxxxxxxxxxxxxxxxxxxxxxxxx"),
        }
    }

    /// Helper to load a single DBC file into its store.
    fn load_dbc<T: DbcEntry>(
        store: &mut DbcStore<T>,
        dbc_path: &Path,
        filename: &str,
    ) -> Result<()> {
        let path = dbc_path.join(filename);
        if !path.exists() {
            warn!("{} not found, skipping", filename);
            return Ok(());
        }
        load_dbc_store(store, path.to_str().unwrap())
            .with_context(|| format!("Failed to load {}", filename))?;
        debug!(
            "Loaded {} {} entries",
            store.len(),
            filename.replace(".dbc", "")
        );
        Ok(())
    }

    /// Helper to load a DBC that logs at debug level when missing.
    fn load_dbc_optional<T: DbcEntry>(
        store: &mut DbcStore<T>,
        dbc_path: &Path,
        filename: &str,
    ) -> Result<()> {
        let path = dbc_path.join(filename);
        if !path.exists() {
            debug!("{} not found, skipping", filename);
            return Ok(());
        }
        load_dbc_store(store, path.to_str().unwrap())
            .with_context(|| format!("Failed to load {}", filename))?;
        debug!(
            "Loaded {} {} entries",
            store.len(),
            filename.replace(".dbc", "")
        );
        Ok(())
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

        Self::load_dbc(&mut self.area_table, dbc_path, "AreaTable.dbc")?;
        Self::load_dbc(&mut self.area_trigger, dbc_path, "AreaTrigger.dbc")?;
        Self::load_dbc(&mut self.auction_house, dbc_path, "AuctionHouse.dbc")?;
        Self::load_dbc(
            &mut self.bank_bag_slot_prices,
            dbc_path,
            "BankBagSlotPrices.dbc",
        )?;
        Self::load_dbc(&mut self.chr_classes, dbc_path, "ChrClasses.dbc")?;
        Self::load_dbc(&mut self.chr_races, dbc_path, "ChrRaces.dbc")?;
        Self::load_dbc(&mut self.map, dbc_path, "Map.dbc")?;

        // Faction.dbc is critical - special error handling
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
                    if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                        error!("IO Error details: {}", io_err);
                    }
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

        Self::load_dbc(&mut self.faction_template, dbc_path, "FactionTemplate.dbc")?;
        Self::load_dbc(
            &mut self.creature_display_info,
            dbc_path,
            "CreatureDisplayInfo.dbc",
        )?;
        Self::load_dbc_optional(&mut self.item, dbc_path, "Item.dbc")?;
        Self::load_dbc(&mut self.skill_line, dbc_path, "SkillLine.dbc")?;
        Self::load_dbc(
            &mut self.skill_line_ability,
            dbc_path,
            "SkillLineAbility.dbc",
        )?;
        Self::load_dbc(&mut self.skill_tiers, dbc_path, "SkillTiers.dbc")?;
        Self::load_dbc(
            &mut self.skill_race_class_info,
            dbc_path,
            "SkillRaceClassInfo.dbc",
        )?;
        Self::load_dbc(&mut self.spell_cast_time, dbc_path, "SpellCastTimes.dbc")?;
        Self::load_dbc(&mut self.spell_duration, dbc_path, "SpellDuration.dbc")?;
        Self::load_dbc(&mut self.spell_radius, dbc_path, "SpellRadius.dbc")?;
        Self::load_dbc(&mut self.spell_range, dbc_path, "SpellRange.dbc")?;
        Self::load_dbc_optional(
            &mut self.spell_focus_object,
            dbc_path,
            "SpellFocusObject.dbc",
        )?;
        Self::load_dbc(&mut self.world_safe_locs, dbc_path, "WorldSafeLocs.dbc")?;
        Self::load_dbc(&mut self.talent, dbc_path, "Talent.dbc")?;
        Self::load_dbc(&mut self.talent_tab, dbc_path, "TalentTab.dbc")?;

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

    /// Get all SkillLineAbility entries linked to a given spell ID.
    /// Corresponds to C++ SpellMgr::GetSkillLineAbilityMapBoundsBySpellId.
    pub fn get_skill_line_abilities_by_spell_id(
        &self,
        spell_id: u32,
    ) -> impl Iterator<Item = &SkillLineAbilityEntry> {
        self.skill_line_ability
            .entries()
            .filter(move |(_, entry)| entry.spell_id == spell_id)
            .map(|(_, entry)| entry)
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
        for (_, entry) in self.skill_race_class_info.entries() {
            if entry.skill_id != skill_id {
                continue;
            }
            if entry.race_mask != 0 {
                let race_bit = 1 << (race - 1);
                if (entry.race_mask & race_bit) == 0 {
                    continue;
                }
            }
            if entry.class_mask != 0 {
                let class_bit = 1 << (class - 1);
                if (entry.class_mask & class_bit) == 0 {
                    continue;
                }
            }
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
}

impl Default for DbcManager {
    fn default() -> Self {
        Self::new()
    }
}
