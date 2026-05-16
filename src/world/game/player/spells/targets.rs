//! Spell Target Resolution
//!
//! Resolves implicit targets from SpellEntry to concrete target lists per effect.
//! Equivalent to MaNGOS Spell::FillTargetMap() / SetTargetMap().
//!
//! Each spell effect has two implicit target fields (target_a, target_b) that
//! describe HOW targets are selected. The client-provided SpellCastTargets
//! supplies the explicit target (who/where the player clicked).

use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::game::player::spells::state::SpellCastTargets;
use crate::world::World;

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
    spell_entry: &crate::world::dbc::structures::SpellEntry,
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
    spell_entry: &crate::world::dbc::structures::SpellEntry,
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
