//! Spell Target Resolution
//!
//! Resolves implicit targets from SpellEntry to concrete target lists per effect.
//! Equivalent to MaNGOS Spell::FillTargetMap() / SetTargetMap().
//!
//! Each spell effect has two implicit target fields (target_a, target_b) that
//! describe HOW targets are selected. The client-provided SpellCastTargets
//! supplies the explicit target (who/where the player clicked).

use crate::game::player::spells::state::SpellCastTargets;
use crate::World;
use oxcore_shared::protocol::{ObjectGuid, Position};

/// MaNGOS implicit target types (from SpellEntry effect_implicit_target fields)
///
/// Only the most common vanilla-relevant values are handled.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplicitTarget {
    None = 0,
    Self_ = 1,
    ChainDamage = 6,
    AllEnemyInArea = 15,
    AllEnemyInAreaInstant = 16,
    EffectSelect = 18,
    AddExtraAttacks = 19,
    AllPartyAroundCaster = 20,
    SingleFriend = 21,
    AllEnemyInAreaChanneled = 22,
    AoETargetEnemy = 24,
    GameObjectItem = 26,
    PetAndSummons = 27,
    AllFriendlyInArea = 30,
    SinglePartyMember = 31,
    AllPartyRange = 33,
    NatureSummon = 34,
    PartyMember = 35,
    AoEPartySrc = 37,
    ChainHeal = 45,
    ScriptTarget = 46,
    SelfFishing = 47,
    GameObjectTarget = 48,
    AllHostileAroundCaster = 52,
    CurrentSelection = 53,
    TargetEnemyNear = 54,
    DynaObjectCoord = 56,
    AllFriendlyAroundCaster = 57,
    AllPartyInArea = 61,
    SingleEnemy = 77,
}

impl ImplicitTarget {
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => Self::None,
            1 => Self::Self_,
            6 => Self::ChainDamage,
            15 => Self::AllEnemyInArea,
            16 => Self::AllEnemyInAreaInstant,
            18 => Self::EffectSelect,
            19 => Self::AddExtraAttacks,
            20 => Self::AllPartyAroundCaster,
            21 => Self::SingleFriend,
            22 => Self::AllEnemyInAreaChanneled,
            24 => Self::AoETargetEnemy,
            26 => Self::GameObjectItem,
            27 => Self::PetAndSummons,
            30 => Self::AllFriendlyInArea,
            31 => Self::SinglePartyMember,
            33 => Self::AllPartyRange,
            34 => Self::NatureSummon,
            35 => Self::PartyMember,
            37 => Self::AoEPartySrc,
            45 => Self::ChainHeal,
            46 => Self::ScriptTarget,
            47 => Self::SelfFishing,
            48 => Self::GameObjectTarget,
            52 => Self::AllHostileAroundCaster,
            53 => Self::CurrentSelection,
            54 => Self::TargetEnemyNear,
            56 => Self::DynaObjectCoord,
            57 => Self::AllFriendlyAroundCaster,
            61 => Self::AllPartyInArea,
            77 => Self::SingleEnemy,
            _ => Self::None,
        }
    }

    /// Whether this target type resolves to the caster
    fn is_self_target(&self) -> bool {
        matches!(self, Self::Self_ | Self::SelfFishing)
    }

    /// Whether this target type is a single explicit target from the client
    fn is_explicit_target(&self) -> bool {
        matches!(
            self,
            Self::EffectSelect
                | Self::SingleEnemy
                | Self::SingleFriend
                | Self::CurrentSelection
                | Self::SinglePartyMember
                | Self::PartyMember
                | Self::GameObjectTarget
                | Self::GameObjectItem
        )
    }

    /// Whether this is an area target
    fn is_area_target(&self) -> bool {
        matches!(
            self,
            Self::AllEnemyInArea
                | Self::AllEnemyInAreaInstant
                | Self::AllEnemyInAreaChanneled
                | Self::AoETargetEnemy
                | Self::AllHostileAroundCaster
                | Self::AllPartyAroundCaster
                | Self::AllFriendlyInArea
                | Self::AllFriendlyAroundCaster
                | Self::AllPartyRange
                | Self::AllPartyInArea
                | Self::AoEPartySrc
        )
    }
}

/// Resolved targets for a single spell cast.
/// Contains per-effect target lists.
#[derive(Debug, Clone)]
pub struct ResolvedTargets {
    /// Target list per effect index (0, 1, 2)
    pub effect_targets: [Vec<ObjectGuid>; 3],
}

impl Default for ResolvedTargets {
    fn default() -> Self {
        Self {
            effect_targets: [Vec::new(), Vec::new(), Vec::new()],
        }
    }
}

/// Resolve spell targets for all effects.
///
/// Reads the spell's implicit target fields and resolves them to concrete GUIDs.
/// Falls back to the explicit target from SpellCastTargets when appropriate.
pub fn resolve_spell_targets(
    spell_id: u32,
    cast_targets: &SpellCastTargets,
    caster_guid: ObjectGuid,
    world: &World,
) -> ResolvedTargets {
    let mut resolved = ResolvedTargets::default();

    let spell_entry = match world.managers.spell_mgr.get(spell_id) {
        Some(entry) => entry,
        None => return resolved,
    };

    for effect_idx in 0..3 {
        if spell_entry.effect[effect_idx] == 0 {
            continue;
        }

        let target_a = ImplicitTarget::from_u32(spell_entry.effect_implicit_target_a[effect_idx]);
        let target_b = ImplicitTarget::from_u32(spell_entry.effect_implicit_target_b[effect_idx]);

        let mut targets = Vec::new();

        // Resolve target_a
        resolve_implicit_target(
            target_a,
            &spell_entry,
            effect_idx,
            cast_targets,
            caster_guid,
            world,
            &mut targets,
        );

        // Resolve target_b (if different from target_a and not None)
        if target_b != ImplicitTarget::None && target_b != target_a {
            resolve_implicit_target(
                target_b,
                &spell_entry,
                effect_idx,
                cast_targets,
                caster_guid,
                world,
                &mut targets,
            );
        }

        // Fallback: if no targets resolved but we have an explicit target, use it
        if targets.is_empty() {
            if let Some(unit_target) = cast_targets.unit_target() {
                targets.push(unit_target);
            } else if target_a == ImplicitTarget::None && target_b == ImplicitTarget::None {
                // No implicit targets and no explicit target = self-cast
                targets.push(caster_guid);
            }
        }

        // Deduplicate
        targets.sort_by_key(|g| g.raw());
        targets.dedup_by_key(|g| g.raw());

        resolved.effect_targets[effect_idx] = targets;
    }

    resolved
}

/// Resolve a single implicit target type into concrete GUIDs.
fn resolve_implicit_target(
    target_type: ImplicitTarget,
    spell_entry: &crate::dbc::structures::SpellEntry,
    effect_idx: usize,
    cast_targets: &SpellCastTargets,
    caster_guid: ObjectGuid,
    world: &World,
    targets: &mut Vec<ObjectGuid>,
) {
    if target_type.is_self_target() {
        targets.push(caster_guid);
        return;
    }

    if target_type.is_explicit_target() {
        // Use the explicit target from the client
        if let Some(guid) = cast_targets.unit_target() {
            targets.push(guid);
        }
        return;
    }

    // Get spell radius for area effects
    let radius = get_effect_radius(spell_entry, effect_idx, world);

    match target_type {
        // Area enemy targets
        ImplicitTarget::AllEnemyInArea
        | ImplicitTarget::AllEnemyInAreaInstant
        | ImplicitTarget::AllEnemyInAreaChanneled
        | ImplicitTarget::AoETargetEnemy => {
            // Get center position: destination from cast_targets, or caster position
            let center = if let Some((x, y, z)) = cast_targets.dst_position {
                Position { x, y, z, o: 0.0 }
            } else {
                get_unit_position(caster_guid, world)
            };

            let nearby = get_units_in_range(caster_guid, center, radius, world);
            for guid in nearby {
                if guid != caster_guid && is_hostile(caster_guid, guid, world) {
                    targets.push(guid);
                }
            }
        }

        // All hostile around caster
        ImplicitTarget::AllHostileAroundCaster | ImplicitTarget::TargetEnemyNear => {
            let center = get_unit_position(caster_guid, world);
            let nearby = get_units_in_range(caster_guid, center, radius, world);
            for guid in nearby {
                if guid != caster_guid && is_hostile(caster_guid, guid, world) {
                    targets.push(guid);
                }
            }
        }

        // Party/friendly area targets
        ImplicitTarget::AllPartyAroundCaster
        | ImplicitTarget::AllPartyRange
        | ImplicitTarget::AllPartyInArea
        | ImplicitTarget::AoEPartySrc => {
            // Include self
            targets.push(caster_guid);
            // TODO: Add party members in range
        }

        ImplicitTarget::AllFriendlyInArea | ImplicitTarget::AllFriendlyAroundCaster => {
            let center = get_unit_position(caster_guid, world);
            let nearby = get_units_in_range(caster_guid, center, radius, world);
            for guid in nearby {
                if !is_hostile(caster_guid, guid, world) {
                    targets.push(guid);
                }
            }
        }

        // Chain targets
        ImplicitTarget::ChainDamage => {
            if let Some(primary) = cast_targets.unit_target() {
                targets.push(primary);
                // TODO: Chain to additional targets based on effect value
            }
        }

        ImplicitTarget::ChainHeal => {
            if let Some(primary) = cast_targets.unit_target() {
                targets.push(primary);
                // TODO: Chain to additional injured friendly targets
            }
        }

        // Script/special targets
        ImplicitTarget::ScriptTarget | ImplicitTarget::DynaObjectCoord => {
            // Use destination position for dynamic object effects
            if let Some(guid) = cast_targets.unit_target() {
                targets.push(guid);
            }
        }

        ImplicitTarget::PetAndSummons => {
            // TODO: Get player's pet GUID
            targets.push(caster_guid);
        }

        ImplicitTarget::AddExtraAttacks | ImplicitTarget::NatureSummon => {
            targets.push(caster_guid);
        }

        _ => {
            // Unknown target type, fall through to explicit target fallback
        }
    }
}

/// Get the radius for an area effect from SpellRadius.dbc.
fn get_effect_radius(
    spell_entry: &crate::dbc::structures::SpellEntry,
    effect_idx: usize,
    world: &World,
) -> f32 {
    let radius_idx = spell_entry.effect_radius_index[effect_idx];
    if radius_idx > 0 {
        let dbc = world.dbc.read();
        if let Some(radius_entry) = dbc.get_spell_radius(radius_idx) {
            return radius_entry.radius;
        }
    }
    // Default radius for area effects
    10.0
}

/// Get a unit's position (player or creature).
fn get_unit_position(guid: ObjectGuid, world: &World) -> Position {
    if guid.is_player() {
        world
            .managers
            .player_mgr
            .with_player(guid, |p| p.movement.position)
            .unwrap_or_default()
    } else if guid.is_creature() {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |c| Position {
                x: c.position.x,
                y: c.position.y,
                z: c.position.z,
                o: c.position.o,
            })
            .unwrap_or_default()
    } else {
        Position::default()
    }
}

/// Get all players and creatures within range of a position on the caster's map.
fn get_units_in_range(
    caster_guid: ObjectGuid,
    center: Position,
    range: f32,
    world: &World,
) -> Vec<ObjectGuid> {
    let (map_id, instance_id) = if caster_guid.is_player() {
        world
            .managers
            .player_mgr
            .with_player(caster_guid, |p| (p.map_id, p.instance_id))
            .unwrap_or((0, 0))
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(caster_guid, |c| (c.map_id, c.instance_id))
            .unwrap_or((0, 0))
    };

    let map = world
        .managers
        .map_mgr
        .get_or_create_map(map_id, instance_id);
    let mut result = map.get_players_in_range(center, range);

    // Also add creatures in range
    let range_sq = range * range;
    map.get_creatures_in_range(center, range_sq, &mut result);

    result
}

/// Check if target is hostile to caster.
/// Simple heuristic: players are not hostile to players (PvP not checked),
/// creatures are always hostile to players (simplified for now).
fn is_hostile(caster_guid: ObjectGuid, target_guid: ObjectGuid, _world: &World) -> bool {
    if caster_guid.is_player() && target_guid.is_creature() {
        return true;
    }
    if caster_guid.is_creature() && target_guid.is_player() {
        return true;
    }
    // TODO: Check faction templates for proper hostility
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::dbc::structures::SpellEntry;
    use crate::game::player::spells::state::TARGET_FLAG_DEST_LOCATION;
    use crate::World;
    use oxcore_shared::database::Databases;
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn lazy_pool() -> sqlx::MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: lazy_pool(),
        });
        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    /// A minimal SpellEntry: one school-damage effect resolved to an instant enemy-AoE,
    /// with a default radius (effect_radius_index 0 → 10.0 yds).
    fn aoe_enemy_spell(id: u32) -> SpellEntry {
        let mut effect = [0u32; 3];
        effect[0] = 2; // SPELL_EFFECT_SCHOOL_DAMAGE
        let mut target_a = [0u32; 3];
        target_a[0] = 16; // TARGET_ENUM_UNITS_ENEMY_AOE_AT_DEST_LOC (AllEnemyInAreaInstant)
        SpellEntry {
            id,
            name: format!("AoE{id}"),
            rank_text: String::new(),
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
            stack_amount: 0,
            totem: [0; 2],
            reagent: [0; 8],
            reagent_count: [0; 8],
            equipped_item_class: 0,
            equipped_item_sub_class_mask: 0,
            equipped_item_inventory_type_mask: 0,
            effect,
            effect_die_sides: [0; 3],
            effect_base_dice: [0; 3],
            effect_dice_per_level: [0.0; 3],
            effect_real_points_per_level: [0.0; 3],
            effect_base_points: [0; 3],
            effect_bonus_coefficient: [0.0; 3],
            effect_mechanic: [0; 3],
            effect_implicit_target_a: target_a,
            effect_implicit_target_b: [0; 3],
            effect_radius_index: [0; 3],
            effect_apply_aura_name: [0; 3],
            effect_amplitude: [0; 3],
            effect_multiple_value: [0.0; 3],
            effect_chain_target: [0; 3],
            effect_item_type: [0; 3],
            effect_misc_value: [0; 3],
            effect_trigger_spell: [0; 3],
            effect_points_per_combo_point: [0.0; 3],
            spell_visual: 0,
            spell_icon_id: 0,
            active_icon_id: 0,
            spell_priority: 0,
            min_target_level: 0,
            mana_cost_percentage: 0,
            start_recovery_category: 0,
            start_recovery_time: 0,
            max_target_level: 0,
            spell_family_name: 0,
            spell_family_flags: 0,
            max_affected_targets: 0,
            dmg_class: 0,
            prevention_type: 0,
            custom: 0,
            internal: 0,
            allowed_target_mask: 0,
            script_id: 0,
            dmg_multiplier: [1.0; 3],
        }
    }

    fn pos(x: f32, y: f32) -> Position {
        Position {
            x,
            y,
            z: 0.0,
            o: 0.0,
        }
    }

    /// Ground-targeted AoE resolves enemies around the destination position, proving the
    /// destination now reaches resolution (the whole point of threading SpellCastTargets).
    #[tokio::test]
    async fn aoe_resolves_enemies_around_destination() {
        let world = test_world();
        world.managers.spell_mgr.add_spell(aoe_enemy_spell(50000));

        // Unregistered player caster → map (0,0), caster position (0,0,0).
        let caster = ObjectGuid::new_player(1);
        let near_origin = ObjectGuid::new_creature(1, 1); // 3 yds from origin
        let near_dest = ObjectGuid::new_creature(1, 2); // at the destination, 100 yds away

        let map = world.managers.map_mgr.get_or_create_map(0, 0);
        map.add_creature(near_origin, pos(3.0, 0.0));
        map.add_creature(near_dest, pos(100.0, 0.0));

        // With a destination at (100,0,0): only the creature at the destination is in radius.
        let with_dst = SpellCastTargets {
            target_flags: TARGET_FLAG_DEST_LOCATION,
            dst_position: Some((100.0, 0.0, 0.0)),
            ..Default::default()
        };
        let resolved = resolve_spell_targets(50000, &with_dst, caster, &world);
        assert!(
            resolved.effect_targets[0].contains(&near_dest),
            "destination AoE should hit the creature at the destination"
        );
        assert!(
            !resolved.effect_targets[0].contains(&near_origin),
            "destination AoE should not hit the far creature near the origin"
        );

        // Without a destination, the AoE centers on the caster (origin): only the near creature.
        let no_dst = SpellCastTargets::default();
        let resolved2 = resolve_spell_targets(50000, &no_dst, caster, &world);
        assert!(
            resolved2.effect_targets[0].contains(&near_origin),
            "origin-centered AoE should hit the creature near the caster"
        );
        assert!(
            !resolved2.effect_targets[0].contains(&near_dest),
            "origin-centered AoE should not hit the distant creature"
        );
    }
}
