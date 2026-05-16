//! Creature Regeneration System
//!
//! Handles health and mana regeneration for all alive creatures.
//! Regeneration ticks every 2 seconds (REGEN_INTERVAL_MS).
//!
//! Health: Out of combat = max_health / 3 per tick. In combat = 0.
//! Mana: Out of combat = max_mana / 3 per tick. In combat = 0 (simplified).

use crate::shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::world::game::broadcast_mgr::broadcast_around_creature;
use crate::world::game::common::update_fields::{
    UNIT_FIELD_HEALTH, UNIT_FIELD_MAXHEALTH, UNIT_FIELD_POWER1,
};
use crate::world::World;

/// Regeneration tick interval: 2 seconds
const REGEN_INTERVAL_MS: u32 = 2000;

/// Process regeneration for all creatures that need it.
/// Called from the world update loop every tick; internally accumulates
/// time and only processes regen every REGEN_INTERVAL_MS.
pub fn update_regeneration(world: &World, diff_ms: u32) {
    // Collect creatures that need regen processing
    let creatures_needing_regen: Vec<ObjectGuid> = world
        .managers
        .creature_mgr
        .iter_creatures()
        .filter(|creature| {
            creature.death_state == super::death::DeathState::Alive
                && (creature.current_health < creature.max_health
                    || (creature.max_mana > 0 && creature.current_mana < creature.max_mana))
        })
        .map(|e| *e.key())
        .collect();

    for creature_guid in creatures_needing_regen {
        // Accumulate regen timer and check if a tick should fire
        let should_regen = world
            .managers
            .creature_mgr
            .with_creature_mut(creature_guid, |creature| {
                creature.regen_timer += diff_ms;
                if creature.regen_timer >= REGEN_INTERVAL_MS {
                    creature.regen_timer -= REGEN_INTERVAL_MS;
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false);

        if should_regen {
            regenerate_creature(world, creature_guid);
        }
    }
}

/// Apply one regeneration tick to a creature and send updates.
fn regenerate_creature(world: &World, creature_guid: ObjectGuid) {
    let regen_result = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |creature| {
            // Re-check death state under lock to avoid race with concurrent kill
            if creature.death_state != super::death::DeathState::Alive {
                return (false, false, creature.current_health, creature.current_mana);
            }

            let in_combat = creature.combat.in_combat;
            let mut health_changed = false;
            let mut mana_changed = false;

            // Health regen: out of combat only
            if !in_combat && creature.current_health < creature.max_health {
                let regen = creature.max_health / 3;
                if regen > 0 {
                    creature.current_health =
                        (creature.current_health + regen).min(creature.max_health);
                    health_changed = true;
                }
            }

            // Mana regen: out of combat only (simplified)
            if !in_combat && creature.max_mana > 0 && creature.current_mana < creature.max_mana {
                let regen = creature.max_mana / 3;
                if regen > 0 {
                    creature.current_mana = (creature.current_mana + regen).min(creature.max_mana);
                    mana_changed = true;
                }
            }

            (
                health_changed,
                mana_changed,
                creature.current_health,
                creature.current_mana,
            )
        });

    let (health_changed, mana_changed, current_health, current_mana) = match regen_result {
        Some(result) => result,
        None => return,
    };

    if !health_changed && !mana_changed {
        return;
    }

    // Build a single update packet with all changed fields
    let entry = creature_guid.entry();
    let world_guid = WorldObjectGuid::new_creature(entry, creature_guid.counter());

    let mut values_block = ValuesUpdateBlock::new(world_guid, ObjectType::Unit);

    if health_changed {
        values_block = values_block.set_field(UNIT_FIELD_HEALTH, current_health);
    }
    if mana_changed {
        values_block = values_block.set_field(UNIT_FIELD_POWER1, current_mana);
    }

    let msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(values_block));

    broadcast_around_creature(world, creature_guid, &msg.to_world_packet());
}
