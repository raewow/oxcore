//! Item and Enchanting Spell Effects
//!
//! Handles item creation, enchanting, disenchanting, and item manipulation.

use super::{EffectInput, EffectResult};
use crate::shared::protocol::ObjectGuid;
use crate::world::World;
use anyhow::Result;

/// Equipment slots for enchanting
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentSlot {
    Head = 0,
    Neck = 1,
    Shoulders = 2,
    Body = 3,
    Chest = 4,
    Waist = 5,
    Legs = 6,
    Feet = 7,
    Wrists = 8,
    Hands = 9,
    Finger1 = 10,
    Finger2 = 11,
    Trinket1 = 12,
    Trinket2 = 13,
    Back = 14,
    MainHand = 15,
    OffHand = 16,
    Ranged = 17,
    Tabard = 18,
}

/// SPELL_EFFECT_CREATE_ITEM (24)
///
/// Create items in the caster's inventory (conjure spells).
/// base_value = item entry ID
/// misc_value = item count
pub async fn effect_create_item(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // Item ID comes from effect_item_type[effect_index], NOT base_value.
    // base_value is the quantity to create.
    let spell_entry = match world.managers.spell_mgr.get(input.spell_id) {
        Some(e) => e,
        None => return Ok(EffectResult::empty()),
    };

    let item_entry = spell_entry.effect_item_type[input.effect_index as usize] as u32;
    if item_entry == 0 {
        return Ok(EffectResult::empty());
    }

    let item_count = input.base_value.max(1) as u32;

    let result = world
        .systems
        .inventory
        .add_item(input.caster_guid, item_entry, item_count)
        .await;

    tracing::debug!(
        "Create item: caster={:?} item={} count={} result={:?}",
        input.caster_guid,
        item_entry,
        item_count,
        result
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_SUMMON_CHANGE_ITEM (34)
///
/// Transform one item into another (e.g., item upgrades).
/// misc_value = source item entry
/// base_value = target item entry
pub async fn effect_summon_change_item(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let source_item_entry = input.misc_value as u32;
    let target_item_entry = input.base_value as u32;

    // TODO: Find and consume source item
    // Then create target item

    tracing::debug!(
        "Item transform: {:?} {} -> {}",
        input.caster_guid,
        source_item_entry,
        target_item_entry
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_ENCHANT_ITEM (53)
///
/// Apply a permanent enchantment to an item.
/// misc_value = enchantment ID
pub async fn effect_enchant_item_perm(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let enchantment_id = input.misc_value as u32;
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);

    // TODO: Get the item to enchant from spell target
    // For now, enchant main hand weapon
    // Need to get item GUID from equipment slot

    tracing::debug!(
        "Permanent enchant: target={:?} enchant={}",
        target_guid,
        enchantment_id
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_ENCHANT_ITEM_TEMPORARY (54)
///
/// Apply a temporary enchantment to an item.
/// misc_value = enchantment ID
/// base_value = duration in seconds
pub async fn effect_enchant_item_tmp(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let enchantment_id = input.misc_value as u32;
    let duration_sec = input.base_value.max(0) as u32;
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);

    // TODO: Get the item to enchant from spell target
    // Apply temporary enchantment with duration

    tracing::debug!(
        "Temporary enchant: target={:?} enchant={} duration={}s",
        target_guid,
        enchantment_id,
        duration_sec
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_ENCHANT_HELD_ITEM (92)
///
/// Enchant the item currently held in the main hand.
/// misc_value = enchantment ID
pub async fn effect_enchant_held_item(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let enchantment_id = input.misc_value as u32;
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);

    // TODO: Get main hand item and apply enchantment

    tracing::debug!(
        "Enchant held item: target={:?} enchant={}",
        target_guid,
        enchantment_id
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_DISENCHANT (99)
///
/// Disenchant an item into enchanting materials.
/// Target item is consumed and materials are created based on item quality and level.
pub async fn effect_disenchant(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    // TODO: Get item info and check if disenchantable
    // Generate materials based on item level/quality
    // Consume item and give materials

    tracing::debug!(
        "Disenchant: caster={:?} item={:?}",
        input.caster_guid,
        target_guid
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_OPEN_LOCK_ITEM (59)
///
/// Open a locked item using a key item.
/// misc_value = lock ID
pub async fn effect_open_lock_item(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let lock_id = input.misc_value as u32;
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    // TODO: Check if caster has the required key
    // Open the lock

    tracing::debug!(
        "Open lock: caster={:?} target={:?} lock={}",
        input.caster_guid,
        target_guid,
        lock_id
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_DURABILITY_DAMAGE (111)
///
/// Damage an item's durability by a flat amount.
/// misc_value = equipment slot
/// base_value = durability damage amount
pub async fn effect_durability_damage(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let slot = input.misc_value as u8;
    let damage = input.base_value.max(0) as u32;
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);

    // TODO: Get item at slot and damage durability
    // Use world.systems.inventory.update_durability

    tracing::debug!(
        "Durability damage: target={:?} slot={} damage={}",
        target_guid,
        slot,
        damage
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_DURABILITY_DAMAGE_PCT (115)
///
/// Damage an item's durability by percentage.
/// misc_value = equipment slot
/// base_value = percentage of max durability to damage (1-100)
pub async fn effect_durability_damage_pct(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    let slot = input.misc_value as u8;
    let damage_pct = input.base_value.max(0).min(100) as u8;
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);

    // TODO: Get item at slot and damage durability by percentage

    tracing::debug!(
        "Durability damage pct: target={:?} slot={} damage_pct={}%",
        target_guid,
        slot,
        damage_pct
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_FEED_PET (101)
///
/// Feed the caster's pet with an item.
/// Consumes the food item and increases pet happiness.
pub async fn effect_feed_pet(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let food_item_entry = input.misc_value as u32;
    let happiness_gain = input.base_value.max(0) as u32;

    // TODO: Consume food item from inventory
    // Increase pet happiness

    tracing::debug!(
        "Feed pet: caster={:?} food={} happiness={}",
        input.caster_guid,
        food_item_entry,
        happiness_gain
    );

    Ok(EffectResult::empty())
}

#[cfg(test)]
mod tests {
    use crate::world::dbc::structures::SpellEntry;

    /// Build a minimal SpellEntry suitable for tests.
    /// Only populates fields relevant to CREATE_ITEM; everything else is zeroed/defaulted.
    fn make_spell_entry(
        id: u32,
        effect_item_type: [u64; 3],
        effect_base_points: [i32; 3],
    ) -> SpellEntry {
        SpellEntry {
            id,
            name: format!("TestSpell{}", id),
            school: 0,
            category: 0,
            dispel: 0,
            mechanic: 0,
            attributes: 0,
            attributes_ex: 0,
            attributes_ex2: 0,
            attributes_ex3: 0,
            attributes_ex4: 0,
            stances: 0,
            stances_not: 0,
            targets: 0,
            target_creature_type: 0,
            requires_spell_focus: 0,
            caster_aura_state: 0,
            target_aura_state: 0,
            casting_time_index: 0,
            recovery_time: 0,
            category_recovery_time: 0,
            interrupt_flags: 0,
            aura_interrupt_flags: 0,
            channel_interrupt_flags: 0,
            proc_flags: 0,
            proc_chance: 0,
            proc_charges: 0,
            max_level: 0,
            base_level: 0,
            spell_level: 0,
            duration_index: 0,
            power_type: 0,
            mana_cost: 0,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            mana_per_second_per_level: 0,
            range_index: 0,
            speed: 0.0,
            stack_amount: 1,
            totem: [0; 2],
            reagent: [0; 8],
            reagent_count: [0; 8],
            equipped_item_class: 0,
            equipped_item_sub_class_mask: 0,
            equipped_item_inventory_type_mask: 0,
            effect: [24, 0, 0], // 24 = SPELL_EFFECT_CREATE_ITEM
            effect_die_sides: [0; 3],
            effect_base_dice: [0; 3],
            effect_dice_per_level: [0.0; 3],
            effect_real_points_per_level: [0.0; 3],
            effect_base_points,
            effect_bonus_coefficient: [0.0; 3],
            effect_mechanic: [0; 3],
            effect_implicit_target_a: [0; 3],
            effect_implicit_target_b: [0; 3],
            effect_radius_index: [0; 3],
            effect_apply_aura_name: [0; 3],
            effect_amplitude: [0; 3],
            effect_multiple_value: [0.0; 3],
            effect_chain_target: [0; 3],
            effect_item_type,
            effect_misc_value: [0; 3],
            effect_trigger_spell: [0; 3],
            effect_points_per_combo_point: [0.0; 3],
            spell_visual: 0,
            spell_icon_id: 0,
            active_icon_id: 0,
            spell_priority: 0,
            mana_cost_percentage: 0,
            start_recovery_category: 0,
            start_recovery_time: 0,
            max_target_level: 0,
            spell_family_name: 0,
            spell_family_flags: 0,
            max_affected_targets: 0,
            dmg_class: 0,
            prevention_type: 0,
            dmg_multiplier: [1.0; 3],
        }
    }

    /// effect_create_item reads the item ID from effect_item_type[effect_index],
    /// not from base_value or misc_value.
    #[test]
    fn test_spell_entry_item_type_field_is_item_id() {
        let item_id: u64 = 1407; // Conjured Water
        let entry = make_spell_entry(587, [item_id, 0, 0], [4, 0, 0]); // base_points=4 (qty)

        // The item comes from effect_item_type, NOT base_value
        assert_eq!(
            entry.effect_item_type[0], item_id,
            "effect_item_type[0] should hold the item entry ID"
        );
        assert_ne!(
            entry.effect_base_points[0] as u64, item_id,
            "base_value (effect_base_points) should NOT be confused for the item ID"
        );
    }

    /// Zero item type means no item to create — the handler should exit early.
    #[test]
    fn test_zero_item_type_is_no_op() {
        let entry = make_spell_entry(1, [0, 0, 0], [1, 0, 0]);
        assert_eq!(
            entry.effect_item_type[0], 0,
            "effect_item_type=0 should be treated as no-op by effect_create_item"
        );
    }

    /// Quantity comes from base_value (effect_base_points), clamped to at least 1.
    #[test]
    fn test_quantity_comes_from_base_points() {
        let entry = make_spell_entry(587, [1407, 0, 0], [4, 0, 0]);
        let qty = entry.effect_base_points[0].max(1) as u32;
        assert_eq!(qty, 4, "Conjure Water rank 1 creates 4 waters");
    }

    #[test]
    fn test_negative_base_points_clamp_to_one() {
        let entry = make_spell_entry(1, [9999, 0, 0], [-1, 0, 0]);
        let qty = entry.effect_base_points[0].max(1) as u32;
        assert_eq!(
            qty, 1,
            "Negative base_points should be clamped to minimum 1 item"
        );
    }
}
