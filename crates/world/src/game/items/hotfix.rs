//! DB2 hotfix records for the 1.14 client
//!
//! The modern client has no item-query opcode. It asks for item and dialogue data as DB2 rows via
//! `CMSG_DB_QUERY_BULK` and expects a `SMSG_DB_REPLY` per record id. This module turns what a 1.12
//! world already knows — `item_template` rows and `npc_text` ids — into those rows.
//!
//! One lookup has no 1.12 source at all and is loaded from CSV under `<data_dir>/db2`: the client wants
//! a `FileDataID` for an item's icon texture, where 1.12 has only a display id, and the mapping is a
//! fixed property of the client's own asset table rather than anything a server knows.
//!
//! It is optional. A missing file costs icons, not function: the client is answered either way, which
//! is what keeps the frame that asked from hanging.
//!
//! Dialogue needs no such file. `npc_text` already carries a `broadcast_text_id` per option and the
//! gossip manager already loads the matching `broadcast_text` rows, so the `BroadcastText` table is
//! served straight from there.

use std::collections::HashMap;
use std::path::Path;

use oxcore_shared::messages::hotfix::ItemHotfixRecord;
use tracing::{info, warn};

use super::manager::ItemTemplate;

/// Lookups that back the DB2 replies, loaded once at startup.
#[derive(Debug, Default)]
pub struct HotfixStore {
    item_icons: HashMap<u32, i32>,
}

impl HotfixStore {
    /// Load the icon mapping from `<data_dir>/db2`, warning but not failing when it is absent.
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("db2").join("item_icons.csv");
        match load_item_icons(&path) {
            Ok(icons) => {
                info!("Loaded {} item icon mappings", icons.len());
                Self { item_icons: icons }
            }
            Err(error) => {
                warn!("No item icon mappings ({error}); modern clients will show blank item icons");
                Self::default()
            }
        }
    }

    /// Build the DB2 view of an item template.
    ///
    /// Both the `Item` and `ItemSparse` rows come from this one record, since the tables overlap and
    /// the client asks for them together.
    pub fn item_record(&self, template: &ItemTemplate) -> ItemHotfixRecord {
        ItemHotfixRecord {
            entry: template.entry,
            name: template.name.clone(),
            description: template.description.clone(),
            class: template.item_class as u8,
            subclass: template.item_subclass as u8,
            material: template.material as u8,
            inventory_type: template.inventory_type,
            required_level: template.required_level as i32,
            sheath_type: template.sheath,
            random_property: template.random_property as u16,
            random_suffix: 0, // A TBC concept; 1.12 has only random_property.
            icon_file_data_id: self
                .item_icons
                .get(&template.display_id)
                .copied()
                .unwrap_or(0),
            max_durability: template.max_durability,
            ammo_type: template.ammo_type,
            damage_types: template.dmg_type,
            armor: template.armor,
            resistances: [
                template.holy_res,
                template.fire_res,
                template.nature_res,
                template.frost_res,
                template.shadow_res,
                template.arcane_res,
            ],
            // 1.12 stores damage as floats and the client's row is integral. Truncating rather than
            // rounding matches what the 1.12 client displayed for the same row.
            damage_min: clamp_damage(&template.dmg_min),
            damage_max: clamp_damage(&template.dmg_max),

            // `allowable_class`/`allowable_race` are -1 for "anyone", and the client reads the same
            // all-bits-set convention, so the sign carries over rather than needing a translation.
            allowed_races: i64::from(template.allowable_race),
            allowed_classes: template.allowable_class as i16,
            duration: template.duration,
            bag_family: template.bag_family,
            ranged_mod: template.range_mod,
            max_stack_size: template.stackable as i32,
            max_count: template.max_count as i32,
            required_spell: template.required_spell,
            sell_price: template.sell_price,
            buy_price: template.buy_price,
            buy_count: template.buy_count,
            flags: template.flags,
            flags_extra: template.extra_flags,
            holiday_id: 0,
            item_limit_category: 0,
            gem_properties: 0,
            socket_bonus: 0,
            totem_category: 0,
            map_id: template.map_bound as u16,
            area_id: template.area_bound as u16,
            item_set: template.set_id as u16,
            lock_id: template.lock_id as u16,
            start_quest_id: template.start_quest as u16,
            page_text: template.page_text as u16,
            delay: template.delay,
            required_rep_faction: template.required_reputation_faction,
            required_skill_level: template.required_skill_rank,
            required_skill_id: template.required_skill,
            item_level: template.item_level as u16,
            socket_colors: [0; 3],
            page_material: template.page_material,
            language: template.page_language,
            bonding: template.bonding,
            stat_types: template.stat_type.map(|stat| stat as i8),
            // The client's stat column is a signed byte, so a 1.12 value outside that range has to be
            // clamped. Letting it wrap turns a large bonus into a large penalty.
            stat_values: template
                .stat_value
                .map(|value| value.clamp(-127, 127) as i8),
            container_slots: template.container_slots,
            required_rep_value: template.required_reputation_rank as u8,
            required_city_rank: template.required_city_rank as u8,
            required_honor_rank: template.required_honor_rank as u8,
            quality: template.quality,
        }
    }
}

/// Damage saturates rather than wrapping: a weapon whose roll exceeds the client's `u16` column
/// should read as the largest hit it can express, not as a near-zero one.
fn clamp_damage(values: &[f32; 5]) -> [u16; 5] {
    values.map(|value| value.clamp(0.0, u16::MAX as f32) as u16)
}

/// `DisplayID,FileDataID`, one pair per line after a header.
fn load_item_icons(path: &Path) -> anyhow::Result<HashMap<u32, i32>> {
    let text = std::fs::read_to_string(path)?;
    let mut icons = HashMap::new();
    for line in text.lines().skip(1) {
        let mut fields = line.split(',');
        let (Some(display_id), Some(file_data_id)) = (fields.next(), fields.next()) else {
            continue;
        };
        if let (Ok(display_id), Ok(file_data_id)) =
            (display_id.trim().parse(), file_data_id.trim().parse())
        {
            icons.insert(display_id, file_data_id);
        }
    }
    Ok(icons)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stat outside the client's signed byte must saturate; wrapping would turn +200 into -56.
    #[test]
    fn an_oversized_stat_saturates_rather_than_wrapping() {
        let template = ItemTemplate {
            stat_value: [200, -200, 0, 0, 0, 0, 0, 0, 0, 0],
            ..Default::default()
        };
        let record = HotfixStore::default().item_record(&template);
        assert_eq!(record.stat_values[0], 127);
        assert_eq!(record.stat_values[1], -127);
    }

    /// "Any class" is -1 in `item_template` and all-bits-set to the client; the sign must survive.
    #[test]
    fn unrestricted_items_stay_unrestricted() {
        let record = HotfixStore::default().item_record(&ItemTemplate::default());
        assert_eq!(record.allowed_classes, -1);
        assert_eq!(record.allowed_races, -1);
    }

    #[test]
    fn a_missing_display_id_yields_no_icon() {
        let record = HotfixStore::default().item_record(&ItemTemplate {
            display_id: 12345,
            ..Default::default()
        });
        assert_eq!(record.icon_file_data_id, 0);
    }
}
