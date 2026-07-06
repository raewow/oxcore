//! Spell Target Resolution
//!
//! Resolves implicit targets from SpellEntry to concrete target lists per effect.
//! Equivalent to MaNGOS Spell::FillTargetMap() / SetTargetMap().
//!
//! Each spell effect has two implicit target fields (target_a, target_b) that
//! describe HOW targets are selected. The client-provided SpellCastTargets
//! supplies the explicit target (who/where the player clicked).

use crate::game::player::auras::effects::AURA_SPELL_MAGNET;
use crate::game::player::spells::state::SpellCastTargets;
use crate::World;
use oxcore_shared::protocol::{ObjectGuid, Position};
use std::collections::HashMap;

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

/// Select the magnet target for a victim, if a spell-magnet aura redirects it.
///
/// If no valid magnet target exists, returns the original victim.
pub fn select_magnet_target(
    victim_guid: ObjectGuid,
    spell_entry: &crate::dbc::structures::SpellEntry,
    world: &World,
) -> ObjectGuid {
    if spell_entry.is_positive_spell() || !victim_guid.is_player() {
        return victim_guid;
    }

    let Some((magnet_guid, aura_spell_id, aura_effect_index)) = world
        .systems
        .player
        .manager()
        .with_player(victim_guid, |victim| {
            victim
                .auras
                .container
                .all_auras()
                .find(|aura| aura.aura_type == AURA_SPELL_MAGNET && aura.caster_guid != victim_guid)
                .map(|aura| (aura.caster_guid, aura.spell_id, aura.effect_index))
        })
        .flatten()
    else {
        return victim_guid;
    };

    let victim_map = world
        .systems
        .player
        .manager()
        .with_player(victim_guid, |victim| (victim.map_id, victim.instance_id))
        .unwrap_or((0, 0));

    let magnet_valid = if magnet_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(magnet_guid, |magnet| {
                magnet.is_alive()
                    && magnet.map_id == victim_map.0
                    && magnet.instance_id == victim_map.1
            })
            .unwrap_or(false)
    } else if magnet_guid.is_creature() {
        world
            .managers
            .creature_mgr
            .with_creature(magnet_guid, |magnet| {
                magnet.current_health > 0
                    && magnet.map_id == victim_map.0
                    && magnet.instance_id == victim_map.1
            })
            .unwrap_or(false)
    } else {
        false
    };

    if !magnet_valid {
        return victim_guid;
    }

    let consumed = world
        .systems
        .player
        .manager()
        .with_player_mut(victim_guid, |victim| {
            let mut depleted = false;
            let mut found = false;
            if let Some(aura) = victim
                .auras
                .container
                .get_aura_mut(aura_spell_id, aura_effect_index)
            {
                depleted = !aura.consume_charge();
                found = true;
            }
            if found {
                if depleted {
                    victim
                        .auras
                        .container
                        .remove_aura(aura_spell_id, aura_effect_index);
                }
                victim.auras.needs_client_update = true;
            }
            found
        })
        .unwrap_or(false);

    if consumed {
        magnet_guid
    } else {
        victim_guid
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
    let mut magnet_cache: HashMap<ObjectGuid, ObjectGuid> = HashMap::new();

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

        // Apply spell magnet redirection once per unique victim for this spell.
        for target in &mut targets {
            let redirected = *magnet_cache
                .entry(*target)
                .or_insert_with(|| select_magnet_target(*target, &spell_entry, world));
            *target = redirected;
        }

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

/// Grounding Totem aura: a target carrying it always passes the creature-type
/// check (it soaks the next hostile spell regardless of the caster's mask).
const AURA_GROUNDING_TOTEM: u32 = 8179;
/// Curse of Doom: only castable on non-player-controlled creatures; when valid
/// it is allowed against every creature type.
const SPELL_CURSE_OF_DOOM: u32 = 603;
/// Dismiss Pet: bypasses the creature-type restriction entirely.
const SPELL_DISMISS_PET: u32 = 2641;
/// Taming Lesson: bypasses the creature-type restriction entirely.
const SPELL_TAMING_LESSON: u32 = 23356;
/// Bitmask covering all 11 vanilla creature types (CREATURE_TYPE_BEAST..GAS_CLOUD).
const CREATURE_TYPE_MASK_ALL: u32 = 0x7FF;

/// Port of `Spell::CheckTargetCreatureType`.
///
/// Returns whether `target_guid` satisfies the spell's `TargetCreatureType`
/// mask, applying the vanilla per-spell special cases. Callers treat `false`
/// as "invalid / immune target" and drop the candidate.
pub fn check_target_creature_type(
    spell_entry: &crate::dbc::structures::SpellEntry,
    target_guid: ObjectGuid,
    world: &World,
) -> bool {
    // Grounding Totem (aura 8179) makes any target valid.
    if unit_has_aura(target_guid, AURA_GROUNDING_TOTEM, world) {
        return true;
    }

    // `GetCharmerOrOwnerPlayerOrPlayerItself() != null` is approximated here as
    // "is a player or a player pet"; charmed creatures are not tracked yet.
    let target_is_player_controlled = target_guid.is_player() || target_guid.is_pet();

    let spell_creature_target_mask = match resolve_creature_target_mask(
        spell_entry.id,
        spell_entry.target_creature_type,
        target_is_player_controlled,
    ) {
        Some(mask) => mask,
        // Curse of Doom on a player-controlled target: reject outright.
        None => return false,
    };

    if spell_creature_target_mask != 0 {
        let target_creature_type_mask = creature_type_mask(target_guid, world);
        return creature_type_allows(spell_creature_target_mask, target_creature_type_mask);
    }

    true
}

/// Applies the per-spell overrides that adjust the effective creature-type mask.
///
/// Returns `None` when the spell must reject the target regardless of mask
/// (Curse of Doom against a player-controlled unit), otherwise the effective
/// mask to test against the target's creature-type bits.
fn resolve_creature_target_mask(
    spell_id: u32,
    base_mask: u32,
    target_is_player_controlled: bool,
) -> Option<u32> {
    match spell_id {
        SPELL_CURSE_OF_DOOM => {
            if target_is_player_controlled {
                None
            } else {
                Some(CREATURE_TYPE_MASK_ALL)
            }
        }
        SPELL_DISMISS_PET | SPELL_TAMING_LESSON => Some(0),
        _ => Some(base_mask),
    }
}

/// The creature-type check itself: a zero target mask (e.g. players, which have
/// no creature type) always passes; otherwise the spell and target bits must
/// overlap.
fn creature_type_allows(spell_mask: u32, target_mask: u32) -> bool {
    target_mask == 0 || (spell_mask & target_mask) != 0
}

/// `Unit::GetCreatureTypeMask()` — `1 << (creature_type - 1)`, or 0 for players
/// and unknown/typeless units.
fn creature_type_mask(guid: ObjectGuid, world: &World) -> u32 {
    if !guid.is_creature_or_pet() {
        return 0;
    }
    world
        .managers
        .creature_mgr
        .with_creature(guid, |c| {
            let ct = c.creature_type;
            if ct > 0 {
                1u32 << (ct - 1)
            } else {
                0
            }
        })
        .unwrap_or(0)
}

/// `Unit::HasAura(spell_id)` for either a player or a creature/pet unit.
fn unit_has_aura(guid: ObjectGuid, spell_id: u32, world: &World) -> bool {
    if guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(guid, |p| p.auras.container.has_aura(spell_id))
            .unwrap_or(false)
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |c| c.has_aura(spell_id))
            .unwrap_or(false)
    }
}

/// `SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED` — casting the spell does not break stealth.
const SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED: u32 = 0x0002_0000;

// Spell icons whose spells are exempt from breaking the caster's stealth outright.
const SPELL_ICON_SHADOWMELD: u32 = 103;
const SPELL_ICON_SAP: u32 = 249;
const SPELL_ICON_CAMOUFLAGE: u32 = 250;
const SPELL_ICON_VANISH: u32 = 252;

// Improved Sap ranks give an escalating chance to stay stealthed after Sapping.
const IMPROVED_SAP_RANK_1: u32 = 14076;
const IMPROVED_SAP_RANK_2: u32 = 14094;
const IMPROVED_SAP_RANK_3: u32 = 14095;

/// Port of `Spell::ShouldRemoveStealthAuras`.
///
/// Decides whether starting this cast should break the caster's stealth. The
/// caller passes `!should_remove_stealth_auras(..)` as the `skip_stealth` flag
/// to the stealth-aura removal — i.e. stealth is removed when this returns
/// `true`. `is_triggered_spell` mirrors `Spell::m_IsTriggeredSpell`.
pub fn should_remove_stealth_auras(
    spell_entry: &crate::dbc::structures::SpellEntry,
    caster_guid: ObjectGuid,
    is_triggered_spell: bool,
    world: &World,
) -> bool {
    // No unit caster → nothing to unstealth.
    if !(caster_guid.is_player() || caster_guid.is_creature_or_pet()) {
        return false;
    }

    if !spell_breaks_stealth_by_default(
        is_triggered_spell,
        spell_entry.attributes_ex,
        spell_entry.spell_icon_id,
    ) {
        return false;
    }

    // Default: remove stealth. Sap (from a player) can retain it via Improved Sap.
    if spell_entry.spell_icon_id == SPELL_ICON_SAP && caster_guid.is_player() {
        let retain_chance = improved_sap_retain_chance(
            unit_has_aura(caster_guid, IMPROVED_SAP_RANK_1, world),
            unit_has_aura(caster_guid, IMPROVED_SAP_RANK_2, world),
            unit_has_aura(caster_guid, IMPROVED_SAP_RANK_3, world),
        );
        if let Some(chance) = retain_chance {
            // Retain stealth `chance`% of the time, otherwise remove it.
            return !roll_chance_u(chance);
        }
    }

    true
}

/// The deterministic gate of `ShouldRemoveStealthAuras`: whether a cast enters
/// the stealth-removal path at all. Triggered spells, spells flagged
/// `ALLOW_WHILE_STEALTHED`, and the dedicated stealth spells (Shadowmeld,
/// Camouflage, Vanish) never break stealth. Sap is intentionally not exempt —
/// it enters the path and is handled by the Improved Sap chance.
fn spell_breaks_stealth_by_default(
    is_triggered: bool,
    attributes_ex: u32,
    spell_icon_id: u32,
) -> bool {
    if is_triggered {
        return false;
    }
    if attributes_ex & SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED != 0 {
        return false;
    }
    !matches!(
        spell_icon_id,
        SPELL_ICON_SHADOWMELD | SPELL_ICON_CAMOUFLAGE | SPELL_ICON_VANISH
    )
}

/// The chance (in percent) that Improved Sap lets the caster keep stealth,
/// given which rank aura is present. Ranks are mutually exclusive in practice
/// and checked rank-1-first, matching the C++ if/else-if chain.
fn improved_sap_retain_chance(has_rank1: bool, has_rank2: bool, has_rank3: bool) -> Option<u32> {
    if has_rank1 {
        Some(30)
    } else if has_rank2 {
        Some(60)
    } else if has_rank3 {
        Some(90)
    } else {
        None
    }
}

/// `roll_chance_u(chance)` — true `chance`% of the time (chance in 0..=100).
fn roll_chance_u(chance: u32) -> bool {
    chance > rand::random::<u32>() % 100
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::dbc::structures::SpellEntry;
    use crate::game::player::auras::aura::{Aura, AuraFlags};
    use crate::game::player::auras::effects::AURA_SPELL_MAGNET;
    use crate::game::player::player::Player;
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

    fn harmful_spell(id: u32) -> SpellEntry {
        aoe_enemy_spell(id)
    }

    fn add_test_player(world: &World, guid: ObjectGuid, map_id: u32, instance_id: u32) {
        let player = Player::new(
            guid,
            format!("P{}", guid.counter()),
            map_id,
            instance_id,
            0,
            60,
            1,
            1,
            0,
        );
        world.managers.player_mgr.add_player(player, guid.counter());
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

    #[tokio::test]
    async fn magnet_target_redirects_and_consumes_last_charge() {
        let world = test_world();
        let victim = ObjectGuid::new_player(10);
        let magnet = ObjectGuid::new_player(11);

        add_test_player(&world, victim, 1, 0);
        add_test_player(&world, magnet, 1, 0);

        world.managers.player_mgr.with_player_mut(victim, |player| {
            player.auras.container.add_aura(Aura::new(
                70000,
                magnet,
                0,
                AURA_SPELL_MAGNET,
                0,
                0,
                Some(1000),
                0,
                1,
                1,
                AuraFlags {
                    is_positive: false,
                    is_negative: true,
                    is_passive: false,
                    can_be_cancelled: false,
                    is_hidden: false,
                    is_permanent: false,
                },
            ));
        });

        let redirected = select_magnet_target(victim, &harmful_spell(50001), &world);
        assert_eq!(redirected, magnet);

        let remaining = world
            .systems
            .player
            .manager()
            .with_player(victim, |player| player.auras.container.has_aura(70000))
            .unwrap_or(false);
        assert!(!remaining, "last aura charge should remove the magnet aura");
    }

    #[tokio::test]
    async fn magnet_target_stays_on_original_victim_when_invalid() {
        let world = test_world();
        let victim = ObjectGuid::new_player(20);
        let magnet = ObjectGuid::new_player(21);

        add_test_player(&world, victim, 1, 0);
        add_test_player(&world, magnet, 2, 0);

        world.managers.player_mgr.with_player_mut(victim, |player| {
            player.auras.container.add_aura(Aura::new(
                70001,
                magnet,
                0,
                AURA_SPELL_MAGNET,
                0,
                0,
                Some(1000),
                0,
                1,
                1,
                AuraFlags {
                    is_positive: false,
                    is_negative: true,
                    is_passive: false,
                    can_be_cancelled: false,
                    is_hidden: false,
                    is_permanent: false,
                },
            ));
        });

        let redirected = select_magnet_target(victim, &harmful_spell(50002), &world);
        assert_eq!(redirected, victim);

        let aura_still_there = world
            .systems
            .player
            .manager()
            .with_player(victim, |player| player.auras.container.has_aura(70001))
            .unwrap_or(false);
        assert!(
            aura_still_there,
            "invalid redirect must not consume aura charges"
        );
    }

    #[test]
    fn creature_type_overlap_rules() {
        // Target with no creature type (players) always passes a non-zero mask.
        assert!(creature_type_allows(0x7FF, 0));
        // Overlapping bits pass.
        assert!(creature_type_allows(0b0010, 0b0011));
        // Disjoint bits fail.
        assert!(!creature_type_allows(0b0100, 0b0011));
    }

    #[test]
    fn creature_target_mask_special_cases() {
        // Ordinary spell keeps its own mask.
        assert_eq!(resolve_creature_target_mask(12345, 0x40, false), Some(0x40));
        // Dismiss Pet / Taming Lesson clear the restriction.
        assert_eq!(resolve_creature_target_mask(SPELL_DISMISS_PET, 0x40, false), Some(0));
        assert_eq!(resolve_creature_target_mask(SPELL_TAMING_LESSON, 0x40, false), Some(0));
        // Curse of Doom: all creature types when target is not player-controlled...
        assert_eq!(
            resolve_creature_target_mask(SPELL_CURSE_OF_DOOM, 0, false),
            Some(CREATURE_TYPE_MASK_ALL)
        );
        // ...but rejected outright against a player-controlled target.
        assert_eq!(resolve_creature_target_mask(SPELL_CURSE_OF_DOOM, 0, true), None);
    }

    #[tokio::test]
    async fn player_target_passes_creature_type_restricted_spell() {
        let world = test_world();
        // Spell restricted to a creature type (e.g. Undead) still allows player
        // targets, whose creature-type mask is 0.
        let mut spell = aoe_enemy_spell(50100);
        spell.target_creature_type = 0x20; // some creature-type bit
        world.managers.spell_mgr.add_spell(spell.clone());

        let player = ObjectGuid::new_player(30);
        add_test_player(&world, player, 1, 0);

        assert!(check_target_creature_type(&spell, player, &world));
    }

    #[test]
    fn stealth_gate_exemptions() {
        // A plain non-triggered spell breaks stealth.
        assert!(spell_breaks_stealth_by_default(false, 0, 0));
        // Triggered spells never break stealth.
        assert!(!spell_breaks_stealth_by_default(true, 0, 0));
        // ALLOW_WHILE_STEALTHED spells never break stealth.
        assert!(!spell_breaks_stealth_by_default(
            false,
            SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED,
            0
        ));
        // The dedicated stealth spells are exempt by icon.
        assert!(!spell_breaks_stealth_by_default(false, 0, SPELL_ICON_SHADOWMELD));
        assert!(!spell_breaks_stealth_by_default(false, 0, SPELL_ICON_CAMOUFLAGE));
        assert!(!spell_breaks_stealth_by_default(false, 0, SPELL_ICON_VANISH));
        // Sap still enters the removal path (handled by the Improved Sap chance).
        assert!(spell_breaks_stealth_by_default(false, 0, SPELL_ICON_SAP));
    }

    #[test]
    fn improved_sap_rank_chances() {
        assert_eq!(improved_sap_retain_chance(false, false, false), None);
        assert_eq!(improved_sap_retain_chance(true, false, false), Some(30));
        assert_eq!(improved_sap_retain_chance(false, true, false), Some(60));
        assert_eq!(improved_sap_retain_chance(false, false, true), Some(90));
        // Rank 1 wins when several are somehow present (matches if/else-if order).
        assert_eq!(improved_sap_retain_chance(true, true, true), Some(30));
    }

    #[tokio::test]
    async fn plain_spell_from_player_removes_stealth() {
        let world = test_world();
        let caster = ObjectGuid::new_player(40);
        add_test_player(&world, caster, 1, 0);

        let spell = aoe_enemy_spell(50200); // attributes_ex 0, icon 0
        assert!(should_remove_stealth_auras(&spell, caster, false, &world));
        // Triggered variant of the same spell does not break stealth.
        assert!(!should_remove_stealth_auras(&spell, caster, true, &world));
    }

    #[tokio::test]
    async fn sap_without_improved_rank_removes_stealth() {
        let world = test_world();
        let caster = ObjectGuid::new_player(41);
        add_test_player(&world, caster, 1, 0);

        let mut sap = aoe_enemy_spell(50201);
        sap.spell_icon_id = SPELL_ICON_SAP;
        // No Improved Sap aura → deterministic: stealth is removed.
        assert!(should_remove_stealth_auras(&sap, caster, false, &world));
    }

    #[tokio::test]
    async fn non_unit_caster_never_removes_stealth() {
        let world = test_world();
        let go_caster = ObjectGuid::new_gameobject(1, 1);
        let spell = aoe_enemy_spell(50202);
        assert!(!should_remove_stealth_auras(&spell, go_caster, false, &world));
    }
}
