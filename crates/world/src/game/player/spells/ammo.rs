//! Ammo consumption for ranged spells.
//!
//! Responsible for deciding and applying the consumption of a single unit of
//! ammunition (or durability / stack) when a ranged spell cast fires.
//!
//! The pure branch cascade is isolated in [`should_consume_ammo`] / [`AmmoAction`]
//! so it can be exercised without a world. The world-coupled entry point
//! [`take_ammo`] reads the player's equipped ranged weapon and
//! [`PLAYER_AMMO_ID`](crate::game::common::update_fields::PLAYER_AMMO_ID) value,
//! then applies the corresponding inventory mutation.

use crate::World;
use oxcore_shared::game::inventory::INVENTORY_SLOT_BAG_0;
use oxcore_shared::protocol::ObjectGuid;

/// Raw `ItemPrototype::SubClass` value for wands. The skill-bitmask constant of
/// the same name elsewhere is `1 << 19`; the prototype stores the plain enum
/// value, so we compare against `19` here.
const ITEM_SUBCLASS_WEAPON_WAND: u32 = 19;

/// `InventoryType == INVTYPE_THROWN` — thrown weapons live in the ranged slot
/// but consume themselves instead of pulling from `PLAYER_AMMO_ID`.
const INVTYPE_THROWN: u8 = 25;

/// `EquipmentSlots::RANGED` — the slot inspected for the ranged weapon.
const EQUIPMENT_SLOT_RANGED: u8 = 17;

/// `WeaponAttackType::RANGED_ATTACK` — the only attack type that consumes ammo.
const RANGED_ATTACK: u32 = 2;

/// Hardcoded spell IDs that never consume ammo, regardless of weapon.
const EXEMPT_SPELL_IDS: [u32; 4] = [2094, 13099, 13119, 23577];

/// Per-cast inputs for [`take_ammo`].
///
/// [`state::ActiveCast`](crate::game::player::spells::state::ActiveCast) carries
/// `spell_id` but not `attack_type`, so the attack type is supplied alongside
/// by the caller.
#[derive(Debug, Clone, Copy)]
pub struct TakeAmmoInput {
    pub spell_id: u32,
    pub attack_type: u32,
}

/// The consumption action computed by the pure decision helper. Encodes the
/// entire `TakeAmmo` branch cascade without touching the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmmoAction {
    /// No consumption: non-player caster, exempt spell, non-ranged attack,
    /// missing weapon, wand, or bow/gun with `PLAYER_AMMO_ID == 0`.
    None,
    /// Thrown weapon with a non-stackable prototype: lose a durability point on
    /// the ranged equipment slot instead of destroying an item.
    DurabilityLossRangedSlot,
    /// Thrown weapon with a stackable prototype: destroy one unit of the
    /// equipped thrown stack.
    DestroyOneStack,
    /// Bow / gun / crossbow: destroy one unit of the ammo item whose entry
    /// equals `PLAYER_AMMO_ID`.
    ConsumeAmmo(u32),
}

/// Bundle of inputs to the pure [`should_consume_ammo`] decision. Every branch
/// of `TakeAmmo` is reachable from these fields alone.
#[derive(Debug, Clone, Copy)]
pub struct AmmoDecisionInput {
    pub is_player: bool,
    pub attack_type_is_ranged: bool,
    pub exempt: bool,
    pub has_weapon: bool,
    pub weapon_subclass_is_wand: bool,
    pub inventory_type_is_thrown: bool,
    pub max_stack_count: u32,
    pub player_ammo_id: u32,
}

/// `true` for the four hardcoded spell IDs that bypass ammo consumption.
pub fn is_ammo_exempt_spell(spell_id: u32) -> bool {
    EXEMPT_SPELL_IDS.contains(&spell_id)
}

/// Pure decision cascade for taking ammo.
///
/// Encodes, in order:
/// 1. non-player caster → [`AmmoAction::None`]
/// 2. exempt spell id → [`AmmoAction::None`]
/// 3. non-ranged attack type → [`AmmoAction::None`]
/// 4. no equipped ranged weapon → [`AmmoAction::None`]
/// 5. wand subclass → [`AmmoAction::None`] (wands don't consume ammo)
/// 6. thrown weapon: `max_stack_count == 1` → [`AmmoAction::DurabilityLossRangedSlot`],
///    otherwise → [`AmmoAction::DestroyOneStack`]
/// 7. bow / gun / crossbow: `player_ammo_id == 0` → [`AmmoAction::None`],
///    otherwise → [`AmmoAction::ConsumeAmmo`] with that entry id.
pub fn should_consume_ammo(input: &AmmoDecisionInput) -> AmmoAction {
    if !input.is_player {
        return AmmoAction::None;
    }
    if input.exempt {
        return AmmoAction::None;
    }
    if !input.attack_type_is_ranged {
        return AmmoAction::None;
    }
    if !input.has_weapon {
        return AmmoAction::None;
    }
    if input.weapon_subclass_is_wand {
        return AmmoAction::None;
    }
    if input.inventory_type_is_thrown {
        if input.max_stack_count == 1 {
            return AmmoAction::DurabilityLossRangedSlot;
        }
        return AmmoAction::DestroyOneStack;
    }
    if input.player_ammo_id == 0 {
        return AmmoAction::None;
    }
    AmmoAction::ConsumeAmmo(input.player_ammo_id)
}

/// World-coupled entry point for ammo consumption.
///
/// Resolves the caster's equipped ranged weapon and `PLAYER_AMMO_ID`, then
/// delegates the branch cascade to [`should_consume_ammo`] and applies its
/// inventory effect. The computed [`AmmoAction`] is returned for observability.
pub async fn take_ammo(
    world: &World,
    caster_guid: ObjectGuid,
    input: &TakeAmmoInput,
) -> AmmoAction {
    // 1. Non-player casters never consume ammo.
    if !caster_guid.is_player() {
        return AmmoAction::None;
    }

    // 2. Hardcoded exempt spell IDs.
    if is_ammo_exempt_spell(input.spell_id) {
        return AmmoAction::None;
    }

    // 3. Only ranged attacks consume ammo.
    if input.attack_type != RANGED_ATTACK {
        return AmmoAction::None;
    }

    // 4. Resolve the equipped ranged weapon (`GetWeaponForAttack(RANGED_ATTACK, ..)`).
    let Some(weapon_guid) = world.systems.inventory.cache().get_item_at(
        caster_guid,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_RANGED,
    ) else {
        return AmmoAction::None;
    };

    let weapon_template = world
        .systems
        .inventory
        .cache()
        .get_item(caster_guid, weapon_guid)
        .and_then(|item| world.managers.item_mgr.get_template(item.read().entry));

    let Some(weapon_template) = weapon_template else {
        return AmmoAction::None;
    };

    // Read PLAYER_AMMO_ID (player.ammo_id).
    let player_ammo_id = world
        .systems
        .player
        .manager()
        .with_player(caster_guid, |p| p.ammo_id)
        .unwrap_or(0);

    let decision = should_consume_ammo(&AmmoDecisionInput {
        is_player: true,
        attack_type_is_ranged: true,
        exempt: false,
        has_weapon: true,
        weapon_subclass_is_wand: weapon_template.item_subclass == ITEM_SUBCLASS_WEAPON_WAND,
        inventory_type_is_thrown: weapon_template.inventory_type == INVTYPE_THROWN,
        max_stack_count: weapon_template.stackable,
        player_ammo_id,
    });

    match decision {
        AmmoAction::None => {}
        AmmoAction::DurabilityLossRangedSlot => {
            let durability = world
                .systems
                .inventory
                .cache()
                .get_item(caster_guid, weapon_guid)
                .map(|item| item.read().durability);
            if let Some(durability) = durability {
                let _ = world
                    .systems
                    .inventory
                    .update_durability(caster_guid, weapon_guid, durability.saturating_sub(1))
                    .await;
            }
        }
        AmmoAction::DestroyOneStack => {
            let _ = world
                .systems
                .inventory
                .remove_item(caster_guid, weapon_guid, 1);
        }
        AmmoAction::ConsumeAmmo(ammo_entry) => {
            world
                .systems
                .inventory
                .destroy_item_count(caster_guid, ammo_entry, 1);
        }
    }

    decision
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> AmmoDecisionInput {
        AmmoDecisionInput {
            is_player: true,
            attack_type_is_ranged: true,
            exempt: false,
            has_weapon: true,
            weapon_subclass_is_wand: false,
            inventory_type_is_thrown: false,
            max_stack_count: 0,
            player_ammo_id: 12345,
        }
    }

    #[test]
    fn exempt_spell_ids_pass_through() {
        for &id in &EXEMPT_SPELL_IDS {
            assert!(is_ammo_exempt_spell(id), "spell {} should be exempt", id);
        }
        // A nearby non-exempt spell is not flagged.
        assert!(!is_ammo_exempt_spell(2095));
    }

    #[test]
    fn exempt_spells_produce_no_action() {
        for &id in &EXEMPT_SPELL_IDS {
            let mut input = base_input();
            input.exempt = is_ammo_exempt_spell(id);
            assert_eq!(should_consume_ammo(&input), AmmoAction::None);
        }
    }

    #[test]
    fn non_player_caster_returns_none() {
        let mut input = base_input();
        input.is_player = false;
        assert_eq!(should_consume_ammo(&input), AmmoAction::None);
    }

    #[test]
    fn non_ranged_attack_returns_none() {
        let mut input = base_input();
        input.attack_type_is_ranged = false;
        assert_eq!(should_consume_ammo(&input), AmmoAction::None);
    }

    #[test]
    fn missing_weapon_returns_none() {
        let mut input = base_input();
        input.has_weapon = false;
        assert_eq!(should_consume_ammo(&input), AmmoAction::None);
    }

    #[test]
    fn wand_weapon_returns_none() {
        let mut input = base_input();
        input.weapon_subclass_is_wand = true;
        assert_eq!(should_consume_ammo(&input), AmmoAction::None);
    }

    #[test]
    fn thrown_non_stackable_durability_loss() {
        let mut input = base_input();
        input.inventory_type_is_thrown = true;
        input.max_stack_count = 1;
        assert_eq!(
            should_consume_ammo(&input),
            AmmoAction::DurabilityLossRangedSlot
        );
    }

    #[test]
    fn thrown_stackable_destroy_one_stack() {
        let mut input = base_input();
        input.inventory_type_is_thrown = true;
        input.max_stack_count = 200;
        assert_eq!(should_consume_ammo(&input), AmmoAction::DestroyOneStack);
    }

    #[test]
    fn bow_with_ammo_consumes_ammo_entry() {
        let input = base_input();
        assert_eq!(should_consume_ammo(&input), AmmoAction::ConsumeAmmo(12345));
    }

    #[test]
    fn bow_without_ammo_returns_none() {
        let mut input = base_input();
        input.player_ammo_id = 0;
        assert_eq!(should_consume_ammo(&input), AmmoAction::None);
    }

    #[test]
    fn exempt_is_checked_before_attack_type() {
        // An exempt spell with a non-ranged attack still yields None (order is
        // irrelevant for the result, but both gates are independently fatal).
        let mut input = base_input();
        input.exempt = true;
        input.attack_type_is_ranged = false;
        assert_eq!(should_consume_ammo(&input), AmmoAction::None);
    }

    #[test]
    fn thrown_stack_count_zero_falls_through_to_destroy() {
        // Only `== 1` triggers durability loss.
        let mut input = base_input();
        input.inventory_type_is_thrown = true;
        input.max_stack_count = 0;
        assert_eq!(should_consume_ammo(&input), AmmoAction::DestroyOneStack);
    }

    fn build_dummy_world() -> World {
        use crate::config::Config;
        use oxcore_db::database::Databases;
        use sqlx::mysql::MySqlPoolOptions;
        use std::path::PathBuf;
        use std::sync::Arc;

        let pool = MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool");
        let databases = Arc::new(Databases {
            world: pool.clone(),
            character: pool.clone(),
            auth: pool.clone(),
            logs: oxcore_db::database::lazy_logs_pool(),
        });
        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    #[tokio::test]
    async fn take_ammo_input_non_player_caster_returns_none() {
        let world = build_dummy_world();
        // A creature GUID is not a player caster.
        let creature = ObjectGuid::new_creature(1, 1);
        let input = TakeAmmoInput {
            spell_id: 123,
            attack_type: RANGED_ATTACK,
        };
        assert_eq!(take_ammo(&world, creature, &input).await, AmmoAction::None);
    }

    #[tokio::test]
    async fn take_ammo_exempt_spell_short_circuits_before_inventory() {
        // Even with no player loaded and no inventory, exempt spells return None.
        let world = build_dummy_world();
        for &id in &EXEMPT_SPELL_IDS {
            let input = TakeAmmoInput {
                spell_id: id,
                attack_type: RANGED_ATTACK,
            };
            // No player registered -> with_player returns None, but the exempt
            // gate fires first so the action is None regardless.
            assert_eq!(
                take_ammo(&world, ObjectGuid::new_player(999), &input).await,
                AmmoAction::None
            );
        }
    }

    #[tokio::test]
    async fn take_ammo_non_ranged_attack_returns_none() {
        let world = build_dummy_world();
        let input = TakeAmmoInput {
            spell_id: 123,
            attack_type: 0, // BASE_ATTACK
        };
        assert_eq!(
            take_ammo(&world, ObjectGuid::new_player(1), &input).await,
            AmmoAction::None
        );
    }
}
