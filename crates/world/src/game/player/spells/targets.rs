//! Spell Target Resolution
//!
//! Resolves implicit targets from SpellEntry to concrete target lists per effect.
//! Equivalent to MaNGOS Spell::FillTargetMap() / SetTargetMap().
//!
//! Each spell effect has two implicit target fields (target_a, target_b) that
//! describe HOW targets are selected. The client-provided SpellCastTargets
//! supplies the explicit target (who/where the player clicked).

use crate::game::common::unit_flags;
use crate::game::player::auras::effects::{AURA_MOD_CHARM, AURA_MOD_POSSESS, AURA_SPELL_MAGNET};
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

/// `Spell::IsAreaAuraEffect` — whether a single effect value is one of the five
/// `SPELL_EFFECT_APPLY_AREA_AURA_*` variants. Used (per effect, not "any effect")
/// by `FillTargetMap`'s caster pre-registration and target-reuse gating.
fn is_area_aura_effect(effect: u32) -> bool {
    matches!(
        effect,
        35   // APPLY_AREA_AURA_PARTY
            | 119  // APPLY_AREA_AURA_PET
            | 128  // APPLY_AREA_AURA_FRIEND
            | 129  // APPLY_AREA_AURA_ENEMY
            | 132 // APPLY_AREA_AURA_RAID
    )
}

/// Port of the `Spell::FillTargetMap` caster self-registration gate (chunk_0).
///
/// When casting with a unit caster, an effect whose implicit targets don't need
/// resolving because the caster itself is always the target (area auras) or
/// where the effect inherently targets the caster's own presence (spawn,
/// language, quest-complete) is pre-registered immediately, bypassing the
/// normal `SetTargetMap` dispatch entirely.
///
/// `effect` is `m_spellInfo->Effect[i]` for the effect slot being processed.
pub fn effect_self_registers_caster(effect: u32) -> bool {
    const SPELL_EFFECT_SPAWN: u32 = 56;
    const SPELL_EFFECT_LANGUAGE: u32 = 27;
    const SPELL_EFFECT_QUEST_COMPLETE: u32 = 24;
    is_area_aura_effect(effect)
        || matches!(
            effect,
            SPELL_EFFECT_SPAWN | SPELL_EFFECT_LANGUAGE | SPELL_EFFECT_QUEST_COMPLETE
        )
}

/// One already-resolved effect's implicit target pair and effect value, as
/// recorded on a prior loop iteration of `FillTargetMap`. Used by
/// [`can_reuse_targets`] to decide whether effect `i` can copy effect `j`'s
/// target list instead of recomputing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectTargetShape {
    pub effect: u32,
    pub target_a: u32,
    pub target_b: u32,
}

/// Port of the `Spell::FillTargetMap` target-reuse gate (chunk_0).
///
/// `MaxAffectedTargets > 1` spells try to reuse an earlier effect's resolved
/// target list instead of re-running implicit-target resolution, provided the
/// two effects share the same implicit-target pair, effect `j` is a real
/// effect (not `SPELL_EFFECT_NONE`), and neither effect is an area aura
/// (area auras always pre-register just the caster, so reuse would be wrong).
pub fn can_reuse_targets(
    max_affected_targets: u32,
    effect_i: EffectTargetShape,
    effect_j: EffectTargetShape,
) -> bool {
    const SPELL_EFFECT_NONE: u32 = 0;
    max_affected_targets > 1
        && effect_i.target_a == effect_j.target_a
        && effect_i.target_b == effect_j.target_b
        && effect_j.effect != SPELL_EFFECT_NONE
        && !is_area_aura_effect(effect_i.effect)
        && !is_area_aura_effect(effect_j.effect)
}

/// A minimal miss-condition + effect-mask view of one `m_UniqueTargetInfo`
/// entry, sufficient for the post-loop attribute checks in
/// `FillTargetMap:chunk_1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetHitInfo {
    pub target_guid: ObjectGuid,
    pub miss_condition: crate::game::player::spells::hit::SpellMissInfo,
}

/// The effect-mask and deletion state shared by a resolved unit, game-object,
/// or item target entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellEffectTarget {
    pub effect_mask: u8,
    pub deleted: bool,
}

/// Port of `Spell::HaveTargetsForEffect`.
///
/// Returns whether any non-deleted entry in the unit, game-object, or item
/// target lists applies to `effect_index`. Indices outside an eight-bit effect
/// mask cannot have a matching target.
pub fn have_targets_for_effect(
    effect_index: u8,
    unit_targets: &[SpellEffectTarget],
    game_object_targets: &[SpellEffectTarget],
    item_targets: &[SpellEffectTarget],
) -> bool {
    let Some(effect_bit) = 1u8.checked_shl(u32::from(effect_index)) else {
        return false;
    };

    unit_targets
        .iter()
        .chain(game_object_targets)
        .chain(item_targets)
        .any(|target| !target.deleted && target.effect_mask & effect_bit != 0)
}

/// `SPELL_ATTR_EX2_FAIL_ON_ALL_TARGETS_IMMUNE` — abort the cast if every
/// registered target was immune.
const SPELL_ATTR_EX2_FAIL_ON_ALL_TARGETS_IMMUNE: u32 = 0x0000_0080;
/// `SPELL_ATTR_EX_REQUIRE_ALL_TARGETS` — abort (silently, effect-mask cleared)
/// if the explicit unit target missed.
const SPELL_ATTR_EX_REQUIRE_ALL_TARGETS: u32 = 0x0004_0000;

/// Port of the `SPELL_ATTR_EX2_FAIL_ON_ALL_TARGETS_IMMUNE` post-loop check in
/// `Spell::FillTargetMap` (chunk_1).
///
/// Returns `true` when the cast must be aborted: the attribute is set, the
/// target list is non-empty, and every target's miss condition is
/// `Immune`/`Immune2`.
pub fn all_targets_immune_should_fail(attributes_ex2: u32, targets: &[TargetHitInfo]) -> bool {
    use crate::game::player::spells::hit::SpellMissInfo;
    if attributes_ex2 & SPELL_ATTR_EX2_FAIL_ON_ALL_TARGETS_IMMUNE == 0 {
        return false;
    }
    !targets.is_empty()
        && targets.iter().all(|t| {
            matches!(
                t.miss_condition,
                SpellMissInfo::Immune | SpellMissInfo::Immune2
            )
        })
}

/// Port of the `SPELL_ATTR_EX_REQUIRE_ALL_TARGETS` post-loop check in
/// `Spell::FillTargetMap` (chunk_1).
///
/// Returns `true` when the cast must silently drop every effect (the caller
/// zeroes every effect mask and returns early): the attribute is set and the
/// explicit unit target's own hit entry recorded a non-`None` miss condition.
pub fn explicit_target_miss_should_clear_all(
    attributes_ex: u32,
    explicit_unit_target: Option<ObjectGuid>,
    targets: &[TargetHitInfo],
) -> bool {
    use crate::game::player::spells::hit::SpellMissInfo;
    if attributes_ex & SPELL_ATTR_EX_REQUIRE_ALL_TARGETS == 0 {
        return false;
    }
    let Some(explicit_guid) = explicit_unit_target else {
        return false;
    };
    targets
        .iter()
        .any(|t| t.target_guid == explicit_guid && t.miss_condition != SpellMissInfo::None)
}

// Implicit target-type ids referenced by `check_target` special-cases.
/// `TARGET_UNIT_CASTER` — the effect always targets the caster; skips the
/// creature-type filter and exempts `ONLY_ON_PLAYER`.
const TARGET_UNIT_CASTER: u32 = 1;
/// `TARGET_UNIT_SCRIPT_NEAR_CASTER` — script-selected unit target.
const TARGET_UNIT_SCRIPT_NEAR_CASTER: u32 = 38;
/// `TARGET_GAMEOBJECT_SCRIPT_NEAR_CASTER` — script-selected gameobject target.
const TARGET_GAMEOBJECT_SCRIPT_NEAR_CASTER: u32 = 40;
/// `TARGET_LOCATION_SCRIPT_NEAR_CASTER` — script-selected location target.
const TARGET_LOCATION_SCRIPT_NEAR_CASTER: u32 = 46;

/// `SPELL_ATTR_EX3_NOT_ON_AOE_IMMUNE` — cannot hit creatures flagged immune to AoE.
const SPELL_ATTR_EX3_NOT_ON_AOE_IMMUNE: u32 = 0x0000_0200;
/// `SPELL_ATTR_EX3_ONLY_ON_PLAYER` — the spell may only affect player targets.
const SPELL_ATTR_EX3_ONLY_ON_PLAYER: u32 = 0x0008_0000;

/// `Spell::IsScriptTarget` — implicit-target ids that are resolved by script and
/// therefore bypass the `UNIT_FLAG_NOT_SELECTABLE` rejection.
fn is_script_target(raw_target: u32) -> bool {
    matches!(
        raw_target,
        TARGET_UNIT_SCRIPT_NEAR_CASTER
            | TARGET_GAMEOBJECT_SCRIPT_NEAR_CASTER
            | TARGET_LOCATION_SCRIPT_NEAR_CASTER
    )
}

/// Whether the per-effect creature-type filter is skipped for this effect: it is
/// skipped when the effect's implicit target A is `TARGET_UNIT_CASTER`.
fn creature_type_check_skipped(target_a_raw: u32) -> bool {
    target_a_raw == TARGET_UNIT_CASTER
}

/// Pure `UNIT_FLAG_NOT_SELECTABLE` rejection decision from the selectability
/// block: a not-selectable target is rejected unless the spell is triggered on
/// its own explicit unit target, or either implicit target is a script target.
fn not_selectable_rejects(
    is_triggered: bool,
    target_is_explicit: bool,
    has_not_selectable_flag: bool,
    target_a_raw: u32,
    target_b_raw: u32,
) -> bool {
    (!is_triggered || !target_is_explicit)
        && has_not_selectable_flag
        && !is_script_target(target_a_raw)
        && !is_script_target(target_b_raw)
}

/// Pure `SPELL_ATTR_EX3_ONLY_ON_PLAYER` rejection decision: a non-player target
/// is rejected when the attribute is set and the effect's implicit target A is
/// neither `TARGET_UNIT_SCRIPT_NEAR_CASTER` nor `TARGET_UNIT_CASTER`.
fn only_on_player_rejects(target_is_player: bool, attributes_ex3: u32, target_a_raw: u32) -> bool {
    !target_is_player
        && attributes_ex3 & SPELL_ATTR_EX3_ONLY_ON_PLAYER != 0
        && target_a_raw != TARGET_UNIT_SCRIPT_NEAR_CASTER
        && target_a_raw != TARGET_UNIT_CASTER
}

/// Whether an effect's apply-aura type is a possess/charm effect subject to the
/// extra self / already-charmed / level-cap rejections.
fn is_possess_or_charm_aura(aura_name: u32) -> bool {
    aura_name == AURA_MOD_POSSESS || aura_name == AURA_MOD_CHARM
}

/// The scalar `Spell` member state that `check_target` reads about the *cast*
/// (as opposed to the candidate target). Mirrors the `m_*` fields the C++
/// `Spell::CheckTarget` dereferences.
#[derive(Debug, Clone, Copy)]
pub struct CheckTargetContext {
    /// `m_caster` — the casting object.
    pub caster_guid: ObjectGuid,
    /// Whether `m_casterUnit` is non-null (the caster is a Unit, not a pure
    /// GameObject/DynamicObject caster). Positive-spell and possess/charm blocks
    /// only run for unit casters.
    pub caster_is_unit: bool,
    /// `m_CastItem != nullptr` — the cast was launched from an item.
    pub cast_item_present: bool,
    /// `m_IsTriggeredSpell`.
    pub is_triggered: bool,
    /// `m_targets.getUnitTarget()` — the explicit unit target from the client.
    pub explicit_unit_target: Option<ObjectGuid>,
    /// `m_spellInfo->IsIgnoringCasterAndTargetRestrictions()`.
    pub ignore_caster_target_restrictions: bool,
}

/// Read a unit's level (player or creature/pet), or 0 if unknown.
fn unit_level(guid: ObjectGuid, world: &World) -> u32 {
    if guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(guid, |p| u32::from(p.level))
            .unwrap_or(0)
    } else if guid.is_creature_or_pet() {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |c| u32::from(c.level))
            .unwrap_or(0)
    } else {
        0
    }
}

/// Read a unit's `UNIT_FIELD_FLAGS`, or 0 if unknown.
fn unit_flags_of(guid: ObjectGuid, world: &World) -> u32 {
    if guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(guid, |p| p.unit_flags)
            .unwrap_or(0)
    } else if guid.is_creature_or_pet() {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |c| c.unit_flags)
            .unwrap_or(0)
    } else {
        0
    }
}

/// Port of `Spell::CheckTarget(Unit* target, SpellEffectIndex eff)`.
///
/// Per-effect Unit filter used while filling the target map: returns `false` to
/// reject a candidate `target_guid` for effect `eff`, `true` to keep it.
///
/// Approximations (infrastructure not yet ported, flagged with `// ...`):
/// - `SelectAuraRankForLevel` (positive player-buff rank gating) — treated as a
///   pass; rank selection is not modelled yet.
/// - Duel opponent restriction — duels are not tracked; the check is skipped.
/// - GM invisibility (`VISIBILITY_OFF`) and the GM + non-positive rejection —
///   GM state is not tracked on Player; players are treated as non-GM.
/// - `GetCharmerGuid` / `GetCharmerOrOwnerGuid` — charm ownership is not tracked;
///   treated as no charmer/owner.
/// - `Creature::IsImmuneToAoe` — AoE immunity data is not ported; treated as not
///   immune.
/// - `SpellScript::OnCheckTarget` — no script hook system; the default `true`
///   result is used.
pub fn check_target(
    spell_entry: &crate::dbc::structures::SpellEntry,
    eff: usize,
    target_guid: ObjectGuid,
    ctx: &CheckTargetContext,
    world: &World,
) -> bool {
    let target_a_raw = spell_entry.effect_implicit_target_a[eff];
    let target_b_raw = spell_entry.effect_implicit_target_b[eff];
    let is_positive = spell_entry.is_positive_spell();
    let target_is_caster = target_guid == ctx.caster_guid;
    let target_is_explicit = ctx.explicit_unit_target == Some(target_guid);

    // Positive-spell restriction block: only for a unit caster targeting someone
    // other than itself with a positive spell.
    if ctx.caster_is_unit && !target_is_caster && is_positive {
        // Player buff on a non-explicit target must match the level-appropriate
        // aura rank (untriggered, non-item casts only).
        if ctx.caster_guid.is_player()
            && !ctx.cast_item_present
            && !ctx.is_triggered
            && !target_is_explicit
        {
            // SelectAuraRankForLevel is not ported; the passed spell is assumed
            // to already be the correct rank for the target's level. ...
        }

        // Duel restriction (targetOwner in a duel whose opponent isn't the
        // caster's owner) is skipped: duels are not tracked. ...
    }

    // Creature-type filter, unless this effect always targets the caster.
    if !creature_type_check_skipped(target_a_raw)
        && !check_target_creature_type(spell_entry, target_guid, world)
    {
        return false;
    }

    // Selectability / spawning block, skipped for the caster itself, for a
    // target owned/charmed by the caster, and when the spell ignores caster and
    // target restrictions.
    let target_owner_is_caster = false; // GetCharmerOrOwnerGuid() == caster: charm ownership not tracked. ...
    if !target_is_caster && !target_owner_is_caster && !ctx.ignore_caster_target_restrictions {
        let flags = unit_flags_of(target_guid, world);
        if flags & unit_flags::SPAWNING != 0 {
            return false;
        }
        if not_selectable_rejects(
            ctx.is_triggered,
            target_is_explicit,
            flags & unit_flags::NOT_SELECTABLE != 0,
            target_a_raw,
            target_b_raw,
        ) {
            return false;
        }
    }

    // Player-target GM / visibility restrictions.
    if !target_is_caster && target_guid.is_player() {
        // GM invisibility (VISIBILITY_OFF) and the GM + non-positive rejection
        // both depend on GM state that is not tracked; players are treated as
        // visible non-GMs, so neither rejection fires. ...
    }

    // Possess / charm aura restrictions.
    if is_possess_or_charm_aura(spell_entry.effect_apply_aura_name[eff]) {
        if target_is_caster {
            return false;
        }
        // GetCharmerGuid() != 0 rejection: charm state is not tracked. ...
        let level_cap = spell_entry.effect_base_points[eff];
        if unit_level(target_guid, world) as i32 > level_cap {
            return false;
        }
    }

    // AoE-immune creatures.
    if spell_entry.attributes_ex3 & SPELL_ATTR_EX3_NOT_ON_AOE_IMMUNE != 0
        && target_guid.is_creature()
    {
        let immune_to_aoe = false; // Creature::IsImmuneToAoe not ported. ...
        if immune_to_aoe {
            return false;
        }
    }

    // Player-only spells.
    if only_on_player_rejects(
        target_guid.is_player(),
        spell_entry.attributes_ex3,
        target_a_raw,
    ) {
        return false;
    }

    // m_spellScript->OnCheckTarget would run here; no script hook system yet, so
    // the default result stands. ...
    true
}

/// The scalar `Spell` / cast parameters read by [`fill_raid_or_party_targets`],
/// mirroring the C++ arguments plus the `m_caster` / `m_spellInfo->spellLevel`
/// member state it consults.
#[derive(Debug, Clone, Copy)]
pub struct RaidPartyFillParams {
    /// `m_caster` — used for the hostility and distance gates.
    pub caster_guid: ObjectGuid,
    /// Distance limit for the `IsWithinDistInMap` gate on non-caster units.
    pub radius: f32,
    /// `raid`: include every subgroup when true; otherwise only the anchor's
    /// subgroup.
    pub raid: bool,
    /// `withPets`: also append each eligible player's pet.
    pub with_pets: bool,
    /// `withcaster`: allow the caster itself to be appended without a distance
    /// check.
    pub with_caster: bool,
    /// `m_spellInfo->spellLevel` — the level gate is `level + 10 >= spellLevel`.
    pub spell_level: u32,
}

/// Whether a group member passes the group-path eligibility gate: correct
/// subgroup (or a raid spell), high enough level, and not hostile to the caster.
fn group_member_eligible(
    raid: bool,
    member_subgroup: u8,
    anchor_subgroup: u8,
    member_level: u32,
    spell_level: u32,
    caster_hostile_to_member: bool,
) -> bool {
    (raid || member_subgroup == anchor_subgroup)
        && member_level + 10 >= spell_level
        && !caster_hostile_to_member
}

/// The append decision shared by every unit/pet in both paths: the caster is
/// added only with `withcaster`; anyone else must be within range.
fn unit_passes_distance_gate(is_caster: bool, with_caster: bool, within_dist: bool) -> bool {
    (is_caster && with_caster) || (!is_caster && within_dist)
}

/// Read a unit's (map_id, instance_id), or `None` if unknown.
fn unit_map(guid: ObjectGuid, world: &World) -> Option<(u32, u32)> {
    if guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(guid, |p| (p.map_id, p.instance_id))
    } else if guid.is_creature_or_pet() {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |c| (c.map_id, c.instance_id))
    } else {
        None
    }
}

/// `Unit::IsWithinDistInMap` — same map/instance and within `radius` (centre
/// distance; combat-reach padding is not modelled). ...
fn is_within_dist_in_map(
    caster: ObjectGuid,
    target: ObjectGuid,
    radius: f32,
    world: &World,
) -> bool {
    match (unit_map(caster, world), unit_map(target, world)) {
        (Some(a), Some(b)) if a == b => get_unit_position(caster, world)
            .is_within_range(&get_unit_position(target, world), radius),
        _ => false,
    }
}

/// `Unit::GetCharmerOrOwnerPlayerOrPlayerItself` — a player resolves to itself;
/// charm/pet ownership is not tracked, so non-players resolve to `None`. ...
fn charmer_or_owner_player_or_self(guid: ObjectGuid) -> Option<ObjectGuid> {
    if guid.is_player() {
        Some(guid)
    } else {
        None
    }
}

/// `Unit::GetPet` — pets are not tracked yet, so no pet is ever resolved; the
/// `withPets` branches keep their structure but never append. ...
fn unit_pet(_owner: ObjectGuid, _world: &World) -> Option<ObjectGuid> {
    None
}

/// Port of `Spell::FillRaidOrPartyTargets`.
///
/// Appends the caster's party/raid (or, with no group, the solo owner/self
/// chain) to `out`, applying the subgroup, level, hostility and distance gates.
///
/// Approximations (flagged with `// ...`): pet resolution (`GetPet`) and non-player
/// charm/owner resolution are not tracked; hostility uses the coarse
/// [`is_hostile`] heuristic (players are never hostile to one another here);
/// distance is centre-to-centre without combat-reach padding.
pub fn fill_raid_or_party_targets(
    anchor_target: ObjectGuid,
    params: &RaidPartyFillParams,
    world: &World,
    out: &mut Vec<ObjectGuid>,
) {
    let p_target = charmer_or_owner_player_or_self(anchor_target);
    let group = p_target.and_then(|g| world.systems.group.get_player_group(g));

    match group {
        Some(group) => {
            let anchor_subgroup = p_target
                .and_then(|g| group.get_member(g).map(|m| m.subgroup))
                .unwrap_or(0);
            for member in &group.members {
                let member_guid = member.guid;
                let eligible = group_member_eligible(
                    params.raid,
                    member.subgroup,
                    anchor_subgroup,
                    unit_level(member_guid, world),
                    params.spell_level,
                    is_hostile(params.caster_guid, member_guid, world),
                );
                if eligible {
                    push_unit_and_pet(member_guid, params, world, out);
                }
            }
        }
        None => {
            // Solo fallback: no subgroup/level/hostility gate, only distance.
            let owner_or_self = p_target.unwrap_or(anchor_target);
            push_unit_and_pet(owner_or_self, params, world, out);
        }
    }
}

/// Append `unit` (and, when `withPets`, its pet) to `out` if it passes the
/// caster/distance gate. Shared by both the group and solo paths.
fn push_unit_and_pet(
    unit: ObjectGuid,
    params: &RaidPartyFillParams,
    world: &World,
    out: &mut Vec<ObjectGuid>,
) {
    let is_caster = unit == params.caster_guid;
    let within =
        !is_caster && is_within_dist_in_map(params.caster_guid, unit, params.radius, world);
    if unit_passes_distance_gate(is_caster, params.with_caster, within) {
        out.push(unit);
    }
    if params.with_pets {
        if let Some(pet) = unit_pet(unit, world) {
            let pet_is_caster = pet == params.caster_guid;
            let pet_within = !pet_is_caster
                && is_within_dist_in_map(params.caster_guid, pet, params.radius, world);
            if unit_passes_distance_gate(pet_is_caster, params.with_caster, pet_within) {
                out.push(pet);
            }
        }
    }
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
        assert_eq!(
            resolve_creature_target_mask(SPELL_DISMISS_PET, 0x40, false),
            Some(0)
        );
        assert_eq!(
            resolve_creature_target_mask(SPELL_TAMING_LESSON, 0x40, false),
            Some(0)
        );
        // Curse of Doom: all creature types when target is not player-controlled...
        assert_eq!(
            resolve_creature_target_mask(SPELL_CURSE_OF_DOOM, 0, false),
            Some(CREATURE_TYPE_MASK_ALL)
        );
        // ...but rejected outright against a player-controlled target.
        assert_eq!(
            resolve_creature_target_mask(SPELL_CURSE_OF_DOOM, 0, true),
            None
        );
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
        assert!(!spell_breaks_stealth_by_default(
            false,
            0,
            SPELL_ICON_SHADOWMELD
        ));
        assert!(!spell_breaks_stealth_by_default(
            false,
            0,
            SPELL_ICON_CAMOUFLAGE
        ));
        assert!(!spell_breaks_stealth_by_default(
            false,
            0,
            SPELL_ICON_VANISH
        ));
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
        assert!(!should_remove_stealth_auras(
            &spell, go_caster, false, &world
        ));
    }

    #[test]
    fn area_aura_effects_are_detected() {
        assert!(is_area_aura_effect(35)); // PARTY
        assert!(is_area_aura_effect(119)); // PET
        assert!(is_area_aura_effect(128)); // FRIEND
        assert!(is_area_aura_effect(129)); // ENEMY
        assert!(is_area_aura_effect(132)); // RAID
        assert!(!is_area_aura_effect(2)); // SCHOOL_DAMAGE
    }

    #[test]
    fn caster_self_registration_gate() {
        // Area auras and spawn/language/quest-complete effects self-register.
        assert!(effect_self_registers_caster(35));
        assert!(effect_self_registers_caster(56)); // SPAWN
        assert!(effect_self_registers_caster(27)); // LANGUAGE
        assert!(effect_self_registers_caster(24)); // QUEST_COMPLETE
                                                   // An ordinary damage effect does not.
        assert!(!effect_self_registers_caster(2));
    }

    #[test]
    fn target_reuse_requires_matching_pair_and_no_area_aura() {
        let base = EffectTargetShape {
            effect: 2, // SCHOOL_DAMAGE
            target_a: 6,
            target_b: 0,
        };
        // Identical shape, MaxAffectedTargets > 1: reuse allowed.
        assert!(can_reuse_targets(3, base, base));
        // MaxAffectedTargets <= 1: no reuse.
        assert!(!can_reuse_targets(1, base, base));
        assert!(!can_reuse_targets(0, base, base));
        // Mismatched implicit targets: no reuse.
        let mismatched = EffectTargetShape {
            target_a: 7,
            ..base
        };
        assert!(!can_reuse_targets(3, base, mismatched));
        // Effect j is SPELL_EFFECT_NONE: no reuse (nothing to copy).
        let none_effect = EffectTargetShape { effect: 0, ..base };
        assert!(!can_reuse_targets(3, base, none_effect));
        // Either effect being an area aura disables reuse.
        let area_aura = EffectTargetShape {
            effect: 129,
            ..base
        };
        assert!(!can_reuse_targets(3, area_aura, base));
        assert!(!can_reuse_targets(3, base, area_aura));
    }

    #[test]
    fn target_lists_are_checked_for_requested_effect() {
        let matching = SpellEffectTarget {
            effect_mask: 1 << 1,
            deleted: false,
        };

        assert!(have_targets_for_effect(1, &[matching], &[], &[]));
        assert!(have_targets_for_effect(1, &[], &[matching], &[]));
        assert!(have_targets_for_effect(1, &[], &[], &[matching]));
    }

    #[test]
    fn deleted_or_unmatched_targets_do_not_count() {
        let deleted_match = SpellEffectTarget {
            effect_mask: 1 << 2,
            deleted: true,
        };
        let other_effect = SpellEffectTarget {
            effect_mask: 1 << 1,
            deleted: false,
        };

        assert!(!have_targets_for_effect(
            2,
            &[deleted_match],
            &[other_effect],
            &[deleted_match]
        ));
        assert!(!have_targets_for_effect(2, &[], &[], &[]));
    }

    #[test]
    fn target_effect_mask_uses_the_requested_index() {
        let targets = [SpellEffectTarget {
            effect_mask: (1 << 0) | (1 << 7),
            deleted: false,
        }];

        assert!(have_targets_for_effect(0, &targets, &[], &[]));
        assert!(have_targets_for_effect(7, &targets, &[], &[]));
        assert!(!have_targets_for_effect(1, &targets, &[], &[]));
        assert!(!have_targets_for_effect(8, &targets, &[], &[]));
    }

    #[test]
    fn all_targets_immune_gate() {
        use crate::game::player::spells::hit::SpellMissInfo;
        let immune_targets = vec![
            TargetHitInfo {
                target_guid: ObjectGuid::new_player(1),
                miss_condition: SpellMissInfo::Immune,
            },
            TargetHitInfo {
                target_guid: ObjectGuid::new_player(2),
                miss_condition: SpellMissInfo::Immune2,
            },
        ];
        assert!(all_targets_immune_should_fail(
            SPELL_ATTR_EX2_FAIL_ON_ALL_TARGETS_IMMUNE,
            &immune_targets
        ));
        // Attribute not set: never fails.
        assert!(!all_targets_immune_should_fail(0, &immune_targets));
        // Empty target list: never fails (nothing to be "all immune").
        assert!(!all_targets_immune_should_fail(
            SPELL_ATTR_EX2_FAIL_ON_ALL_TARGETS_IMMUNE,
            &[]
        ));
        // One non-immune target: does not fail.
        let mixed = vec![
            immune_targets[0],
            TargetHitInfo {
                target_guid: ObjectGuid::new_player(3),
                miss_condition: SpellMissInfo::None,
            },
        ];
        assert!(!all_targets_immune_should_fail(
            SPELL_ATTR_EX2_FAIL_ON_ALL_TARGETS_IMMUNE,
            &mixed
        ));
    }

    #[test]
    fn require_all_targets_gate() {
        use crate::game::player::spells::hit::SpellMissInfo;
        let explicit = ObjectGuid::new_player(5);
        let missed = vec![TargetHitInfo {
            target_guid: explicit,
            miss_condition: SpellMissInfo::Resist,
        }];
        assert!(explicit_target_miss_should_clear_all(
            SPELL_ATTR_EX_REQUIRE_ALL_TARGETS,
            Some(explicit),
            &missed
        ));
        // Attribute not set: never clears.
        assert!(!explicit_target_miss_should_clear_all(
            0,
            Some(explicit),
            &missed
        ));
        // No explicit target: never clears.
        assert!(!explicit_target_miss_should_clear_all(
            SPELL_ATTR_EX_REQUIRE_ALL_TARGETS,
            None,
            &missed
        ));
        // Explicit target actually hit: does not clear.
        let hit = vec![TargetHitInfo {
            target_guid: explicit,
            miss_condition: SpellMissInfo::None,
        }];
        assert!(!explicit_target_miss_should_clear_all(
            SPELL_ATTR_EX_REQUIRE_ALL_TARGETS,
            Some(explicit),
            &hit
        ));
    }

    fn ctx_for(caster: ObjectGuid) -> CheckTargetContext {
        CheckTargetContext {
            caster_guid: caster,
            caster_is_unit: true,
            cast_item_present: false,
            is_triggered: false,
            explicit_unit_target: None,
            ignore_caster_target_restrictions: false,
        }
    }

    #[test]
    fn script_target_ids_bypass_not_selectable() {
        assert!(is_script_target(TARGET_UNIT_SCRIPT_NEAR_CASTER));
        assert!(is_script_target(TARGET_GAMEOBJECT_SCRIPT_NEAR_CASTER));
        assert!(is_script_target(TARGET_LOCATION_SCRIPT_NEAR_CASTER));
        assert!(!is_script_target(TARGET_UNIT_CASTER));
        assert!(!is_script_target(16));
    }

    #[test]
    fn creature_type_filter_is_skipped_only_for_caster_target() {
        assert!(creature_type_check_skipped(TARGET_UNIT_CASTER));
        assert!(!creature_type_check_skipped(16));
    }

    #[test]
    fn not_selectable_rejection_rules() {
        // Non-selectable target with a plain (non-script) target: rejected.
        assert!(not_selectable_rejects(false, false, true, 16, 0));
        // Not flagged non-selectable: never rejected.
        assert!(!not_selectable_rejects(false, false, false, 16, 0));
        // Triggered spell on its own explicit target: allowed through.
        assert!(!not_selectable_rejects(true, true, true, 16, 0));
        // Triggered but not the explicit target: still rejected.
        assert!(not_selectable_rejects(true, false, true, 16, 0));
        // Script target A or B exempts the rejection.
        assert!(!not_selectable_rejects(
            false,
            false,
            true,
            TARGET_UNIT_SCRIPT_NEAR_CASTER,
            0
        ));
        assert!(!not_selectable_rejects(
            false,
            false,
            true,
            16,
            TARGET_LOCATION_SCRIPT_NEAR_CASTER
        ));
    }

    #[test]
    fn only_on_player_rejection_rules() {
        // Non-player target, attribute set, ordinary target A: rejected.
        assert!(only_on_player_rejects(
            false,
            SPELL_ATTR_EX3_ONLY_ON_PLAYER,
            16
        ));
        // Player target: never rejected.
        assert!(!only_on_player_rejects(
            true,
            SPELL_ATTR_EX3_ONLY_ON_PLAYER,
            16
        ));
        // Attribute not set: never rejected.
        assert!(!only_on_player_rejects(false, 0, 16));
        // Exempt implicit target ids.
        assert!(!only_on_player_rejects(
            false,
            SPELL_ATTR_EX3_ONLY_ON_PLAYER,
            TARGET_UNIT_CASTER
        ));
        assert!(!only_on_player_rejects(
            false,
            SPELL_ATTR_EX3_ONLY_ON_PLAYER,
            TARGET_UNIT_SCRIPT_NEAR_CASTER
        ));
    }

    #[test]
    fn possess_charm_aura_detection() {
        assert!(is_possess_or_charm_aura(AURA_MOD_POSSESS));
        assert!(is_possess_or_charm_aura(AURA_MOD_CHARM));
        assert!(!is_possess_or_charm_aura(0));
    }

    #[tokio::test]
    async fn check_target_accepts_valid_player_target() {
        let world = test_world();
        let caster = ObjectGuid::new_player(60);
        let target = ObjectGuid::new_player(61);
        add_test_player(&world, caster, 1, 0);
        add_test_player(&world, target, 1, 0);

        let spell = aoe_enemy_spell(51000); // harmful damage spell, target_a = 16
        assert!(check_target(&spell, 0, target, &ctx_for(caster), &world));
    }

    #[tokio::test]
    async fn check_target_rejects_non_player_for_only_on_player_spell() {
        let world = test_world();
        let caster = ObjectGuid::new_player(62);
        let creature = ObjectGuid::new_creature(1, 62);
        add_test_player(&world, caster, 1, 0);

        let mut spell = aoe_enemy_spell(51001);
        spell.attributes_ex3 = SPELL_ATTR_EX3_ONLY_ON_PLAYER;
        assert!(!check_target(&spell, 0, creature, &ctx_for(caster), &world));
    }

    #[tokio::test]
    async fn check_target_possess_rejects_target_above_level_cap() {
        let world = test_world();
        let caster = ObjectGuid::new_player(63);
        let target = ObjectGuid::new_player(64); // level 60 via add_test_player
        add_test_player(&world, caster, 1, 0);
        add_test_player(&world, target, 1, 0);

        let mut spell = aoe_enemy_spell(51002);
        spell.effect_apply_aura_name[0] = AURA_MOD_CHARM;
        // Cap below the target's level (60): rejected.
        spell.effect_base_points[0] = 50;
        assert!(!check_target(&spell, 0, target, &ctx_for(caster), &world));
        // Cap at/above the target's level: passes the level gate (other checks ok).
        spell.effect_base_points[0] = 70;
        assert!(check_target(&spell, 0, target, &ctx_for(caster), &world));
    }

    #[test]
    fn group_member_eligibility_rules() {
        // Same subgroup, adequate level, non-hostile: eligible.
        assert!(group_member_eligible(false, 0, 0, 60, 10, false));
        // Wrong subgroup on a party (non-raid) spell: excluded.
        assert!(!group_member_eligible(false, 1, 0, 60, 10, false));
        // Raid spell ignores subgroup mismatch.
        assert!(group_member_eligible(true, 1, 0, 60, 10, false));
        // Level gate: level + 10 must be >= spellLevel.
        assert!(group_member_eligible(false, 0, 0, 30, 40, false)); // 40 >= 40
        assert!(!group_member_eligible(false, 0, 0, 29, 40, false)); // 39 < 40
                                                                     // Hostile members are excluded.
        assert!(!group_member_eligible(false, 0, 0, 60, 10, true));
    }

    #[test]
    fn distance_gate_rules() {
        // The caster is added only with `withcaster`, never via distance.
        assert!(unit_passes_distance_gate(true, true, false));
        assert!(!unit_passes_distance_gate(true, false, false));
        // Non-caster units are gated purely on distance.
        assert!(unit_passes_distance_gate(false, false, true));
        assert!(!unit_passes_distance_gate(false, true, false));
    }

    fn raid_params(caster: ObjectGuid) -> RaidPartyFillParams {
        RaidPartyFillParams {
            caster_guid: caster,
            radius: 30.0,
            raid: false,
            with_pets: false,
            with_caster: true,
            spell_level: 0,
        }
    }

    #[tokio::test]
    async fn fill_targets_solo_path_appends_owner_within_range() {
        let world = test_world();
        let caster = ObjectGuid::new_player(70);
        let anchor = ObjectGuid::new_player(71);
        add_test_player(&world, caster, 1, 0);
        add_test_player(&world, anchor, 1, 0);

        // No group: the anchor (owner/self) is appended when within range.
        let mut out = Vec::new();
        fill_raid_or_party_targets(anchor, &raid_params(caster), &world, &mut out);
        assert_eq!(out, vec![anchor]);
    }

    #[tokio::test]
    async fn fill_targets_solo_path_respects_with_caster() {
        let world = test_world();
        let caster = ObjectGuid::new_player(72);
        add_test_player(&world, caster, 1, 0);

        // Anchor is the caster itself; without `withcaster` it is dropped.
        let mut params = raid_params(caster);
        params.with_caster = false;
        let mut out = Vec::new();
        fill_raid_or_party_targets(caster, &params, &world, &mut out);
        assert!(out.is_empty());

        // With `withcaster` it is added despite the (self) distance being skipped.
        params.with_caster = true;
        let mut out2 = Vec::new();
        fill_raid_or_party_targets(caster, &params, &world, &mut out2);
        assert_eq!(out2, vec![caster]);
    }

    #[tokio::test]
    async fn fill_targets_group_path_filters_by_subgroup() {
        use oxcore_shared::game::group::GroupData;

        let world = test_world();
        let leader = ObjectGuid::new_player(80);
        let same_sub = ObjectGuid::new_player(81);
        let other_sub = ObjectGuid::new_player(82);
        add_test_player(&world, leader, 1, 0);
        add_test_player(&world, same_sub, 1, 0);
        add_test_player(&world, other_sub, 1, 0);

        let mut group = GroupData::new(500, leader, "Leader".to_string());
        group.add_member(same_sub, "SameSub".to_string()).unwrap();
        group.add_member(other_sub, "OtherSub".to_string()).unwrap();
        group.get_member_mut(other_sub).unwrap().subgroup = 1;
        world.systems.group.add_group_test(group);
        world.systems.group.add_player_to_group_test(leader, 500);
        world.systems.group.add_player_to_group_test(same_sub, 500);
        world.systems.group.add_player_to_group_test(other_sub, 500);

        // Party (non-raid) spell anchored on the leader (subgroup 0): only the
        // leader and the same-subgroup member are collected.
        let mut out = Vec::new();
        fill_raid_or_party_targets(leader, &raid_params(leader), &world, &mut out);
        out.sort_by_key(|g| g.raw());
        assert!(out.contains(&leader));
        assert!(out.contains(&same_sub));
        assert!(!out.contains(&other_sub));

        // A raid spell pulls in the other subgroup as well.
        let mut raid = raid_params(leader);
        raid.raid = true;
        let mut out_raid = Vec::new();
        fill_raid_or_party_targets(leader, &raid, &world, &mut out_raid);
        assert!(out_raid.contains(&other_sub));
    }
}
