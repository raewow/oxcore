//! Picking the victim of a creature that is in combat.
//!
//! Each update the creature prunes its threat list (targets that died, left the
//! world or its dungeon copy are dropped), checks that the fight is still inside
//! its threat area, and picks a victim: an active taunt first, otherwise the
//! threat leader by the 110%/130% rule. No victim means the creature evades.

use super::aggro::{attack_distance, threat_area_radius};
use super::snapshot::TargetSnapshot;
use super::units::{
    self, Attacker, UnitInfo, SPELL_AURA_MOD_DETECT_RANGE, SPELL_AURA_MOD_TAUNT,
};
use crate::game::common::creature_flags::{
    CREATURE_FLAG_EXTRA_NO_LEASH_EVADE, CREATURE_FLAG_EXTRA_NO_THREAT_LIST,
};
use crate::game::creature::combat::{VictimCheck, VictimSelection};
use crate::World;
use oxcore_shared::protocol::{ObjectGuid, Position};

/// How long after the last hit or crowd control the threat area is not enforced.
pub const LEASH_EXTENSION_MS: u64 = 12_000;

/// Result of victim selection for one update.
#[derive(Debug, Clone, Default)]
pub struct HostileSelection {
    pub victim: Option<TargetSnapshot>,
    /// Every threat-list target that is still in the creature's copy.
    pub targets: Vec<TargetSnapshot>,
    /// The creature should leave combat and walk home.
    pub evade: bool,
}

struct SelfInfo {
    attacker: Attacker,
    position: Position,
    combat_reach: f32,
    level: u8,
    detection_range: f32,
    detect_range_mod: i32,
    combat_start: Position,
    last_leash_extension_ms: u64,
    flags_extra: u32,
    threat_targets: Vec<ObjectGuid>,
    taunters: Vec<ObjectGuid>,
    current_victim: Option<ObjectGuid>,
    pacified: bool,
    charmed: bool,
}

/// Whether the fight has drifted outside the creature's threat area.
///
/// The area is measured from where the fight started; the creature stays engaged
/// while either it or its victim is inside, and for twelve seconds after the last
/// hit or crowd control. Dungeon creatures and those flagged never to leash have
/// no threat area.
pub fn is_out_of_threat_area(
    creature_position: &Position,
    combat_start: &Position,
    victim_position: &Position,
    attack_distance: f32,
    last_leash_extension_ms: u64,
    now_ms: u64,
    in_dungeon: bool,
    no_leash: bool,
) -> bool {
    if no_leash || in_dungeon {
        return false;
    }
    let radius = threat_area_radius(attack_distance);
    let inside = units::distance(creature_position, combat_start) <= radius
        || units::distance(victim_position, combat_start) <= radius;
    !inside && last_leash_extension_ms + LEASH_EXTENSION_MS < now_ms
}

/// Prune the threat list and pick this update's victim.
pub fn select_hostile_target(world: &World, guid: ObjectGuid, now_ms: u64) -> HostileSelection {
    let Some(me) = world.managers.creature_mgr.with_creature(guid, |c| SelfInfo {
        attacker: Attacker {
            guid,
            map_id: c.map_id,
            instance_id: c.instance_id,
            faction: c.faction,
            unit_flags: c.unit_flags,
            player_controlled: c.owner_guid.is_some_and(|o| o.is_player())
                || c.charm_guid.is_some_and(|o| o.is_player()),
        },
        position: c.position,
        combat_reach: c.combat_reach,
        level: c.level,
        detection_range: c.ai_profile.detection_range,
        detect_range_mod: c.auras.get_total_aura_modifier(SPELL_AURA_MOD_DETECT_RANGE),
        combat_start: c.ai_state_data.combat_start.unwrap_or(c.position),
        last_leash_extension_ms: c.ai_state_data.last_leash_extension_ms(),
        flags_extra: c.ai_profile.flags_extra,
        threat_targets: c
            .threat_manager
            .get_threat_list()
            .into_iter()
            .map(|(target, _)| target)
            .collect(),
        taunters: c
            .auras
            .get_auras_by_type(SPELL_AURA_MOD_TAUNT)
            .into_iter()
            .map(|aura| aura.caster_guid)
            .collect(),
        current_victim: c.combat.attacking,
        pacified: c.ai_state_data.pacified_until_ms > now_ms,
        charmed: c.charm_guid.is_some(),
    }) else {
        return HostileSelection::default();
    };

    if me.pacified {
        return HostileSelection::default();
    }

    // Resolve every target once; drop the ones that died or left this copy.
    let mut resolved: Vec<UnitInfo> = Vec::with_capacity(me.threat_targets.len());
    let mut gone = Vec::new();
    for target in &me.threat_targets {
        match units::resolve_unit(world, *target) {
            Some(info)
                if info.alive
                    && info.map_id == me.attacker.map_id
                    && info.instance_id == me.attacker.instance_id =>
            {
                resolved.push(info)
            }
            _ => gone.push(*target),
        }
    }
    if !gone.is_empty() {
        world.managers.creature_mgr.with_creature_mut(guid, |c| {
            for target in &gone {
                c.threat_manager.remove_target(*target);
                c.combat.remove_attacker(*target);
                if c.combat.attacking == Some(*target) {
                    c.combat.attacking = None;
                }
            }
        });
    }

    let targets: Vec<TargetSnapshot> = resolved.iter().map(snapshot_of).collect();
    let in_dungeon = crate::game::instance::lifecycle::is_dungeon_map(world, me.attacker.map_id);
    let no_leash = me.flags_extra & CREATURE_FLAG_EXTRA_NO_LEASH_EVADE != 0;

    let out_of_area = |target: &UnitInfo| {
        let distance = attack_distance(
            me.detection_range,
            me.level,
            target.level,
            me.detect_range_mod,
            target.detected_range_mod,
        );
        is_out_of_threat_area(
            &me.position,
            &me.combat_start,
            &target.position,
            distance,
            me.last_leash_extension_ms,
            now_ms,
            in_dungeon,
            no_leash,
        )
    };

    // A living, attackable taunter takes the victim slot outright.
    let taunt_victim = me.taunters.iter().rev().find_map(|caster| {
        resolved
            .iter()
            .find(|t| t.guid == *caster && units::is_valid_attack_target(world, &me.attacker, t))
    });

    let selection = if let Some(taunter) = taunt_victim {
        VictimSelection::Victim(taunter.guid)
    } else {
        let mut checks = std::collections::HashMap::new();
        for target in &resolved {
            let check = if out_of_area(target) {
                VictimCheck::OutOfThreatArea
            } else if units::is_valid_attack_target(world, &me.attacker, target) {
                VictimCheck::Valid {
                    in_melee: units::in_melee_range(&me.position, me.combat_reach, target),
                }
            } else {
                VictimCheck::Invalid
            };
            checks.insert(target.guid, check);
        }
        world
            .managers
            .creature_mgr
            .with_creature_mut(guid, |c| {
                c.threat_manager.select_next_victim(|target| {
                    checks.get(&target).copied().unwrap_or(VictimCheck::Invalid)
                })
            })
            .unwrap_or(VictimSelection::None)
    };

    match selection {
        VictimSelection::Victim(victim) => HostileSelection {
            victim: targets.iter().find(|t| t.guid == victim).cloned(),
            targets,
            evade: false,
        },
        VictimSelection::OutOfThreatArea => HostileSelection {
            victim: None,
            targets,
            evade: true,
        },
        VictimSelection::None => {
            // Creatures without a threat list stay on whoever they were fighting.
            if me.flags_extra & CREATURE_FLAG_EXTRA_NO_THREAT_LIST != 0 {
                if let Some(victim) = me
                    .current_victim
                    .and_then(|v| resolved.iter().find(|t| t.guid == v))
                    .filter(|t| units::is_valid_attack_target(world, &me.attacker, t))
                {
                    return HostileSelection {
                        victim: Some(snapshot_of(victim)),
                        targets,
                        evade: false,
                    };
                }
            }
            // A charmed or taunted creature never evades on its own.
            let held = me.charmed || !me.taunters.is_empty();
            HostileSelection {
                victim: None,
                targets,
                evade: !held,
            }
        }
    }
}

fn snapshot_of(info: &UnitInfo) -> TargetSnapshot {
    TargetSnapshot {
        guid: info.guid,
        position: info.position,
        is_alive: info.alive,
        is_player: info.is_player,
        health_pct: info.health_pct,
        has_mana: info.has_mana,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: f32) -> Position {
        Position::new(x, 0.0, 0.0, 0.0)
    }

    #[test]
    fn inside_the_area_never_leashes() {
        assert!(!is_out_of_threat_area(&pos(40.0), &pos(0.0), &pos(41.0), 18.0, 0, 100_000, false, false));
    }

    #[test]
    fn victim_inside_keeps_the_creature_engaged() {
        assert!(!is_out_of_threat_area(&pos(80.0), &pos(0.0), &pos(30.0), 18.0, 0, 100_000, false, false));
    }

    #[test]
    fn both_outside_leash_after_the_extension_expires() {
        assert!(is_out_of_threat_area(&pos(80.0), &pos(0.0), &pos(81.0), 18.0, 0, 100_000, false, false));
        assert!(
            !is_out_of_threat_area(&pos(80.0), &pos(0.0), &pos(81.0), 18.0, 95_000, 100_000, false, false),
            "a hit in the last twelve seconds holds the leash"
        );
    }

    #[test]
    fn dungeons_and_no_leash_creatures_have_no_threat_area() {
        assert!(!is_out_of_threat_area(&pos(500.0), &pos(0.0), &pos(501.0), 18.0, 0, 100_000, true, false));
        assert!(!is_out_of_threat_area(&pos(500.0), &pos(0.0), &pos(501.0), 18.0, 0, 100_000, false, true));
    }
}
