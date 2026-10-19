//! CMSG_PET_ACTION: the pet action bar — stay, follow, attack, dismiss, the
//! passive/defensive/aggressive stance, and pet spells.

use crate::core::common::packet::WorldPacketGuidExt;
use crate::core::session::WorldSession;
use crate::game::creature::ai::{self, AIAction, AIEvent, ReactState};
use crate::World;
use anyhow::Result;
use oxcore_shared::protocol::{ObjectGuid, Protocol, WorldPacket};

/// Action types in the high byte of the action word.
const ACT_REACTION: u32 = 0x06;
const ACT_COMMAND: u32 = 0x07;

const COMMAND_STAY: u32 = 0;
const COMMAND_FOLLOW: u32 = 1;
const COMMAND_ATTACK: u32 = 2;
const COMMAND_ABANDON: u32 = 3;

const REACT_PASSIVE: u32 = 0;
const REACT_DEFENSIVE: u32 = 1;

/// Pet guid, action word and target guid; the modern client adds a position.
pub fn read_pet_action(protocol: Protocol, packet: &mut WorldPacket) -> Option<(ObjectGuid, u32, ObjectGuid)> {
    let pet = packet.read_guid_for(protocol)?;
    let data = packet.read_u32()?;
    let target = packet.read_guid_for(protocol).unwrap_or_default();
    Some((pet, data, target))
}

pub async fn handle_pet_action(session: &WorldSession, packet: &mut WorldPacket, world: &World) -> Result<()> {
    let Some((pet_guid, data, target)) = read_pet_action(session.protocol(), packet) else {
        return Ok(());
    };
    let Some(player_guid) = session.player_guid() else {
        return Ok(());
    };
    let owned = world
        .managers
        .creature_mgr
        .with_creature(pet_guid, |c| (c.owner_guid == Some(player_guid), c.is_alive()));
    let Some((true, true)) = owned else {
        return Ok(());
    };

    let action = data & 0x00FF_FFFF;
    let kind = (data & 0xFF00_0000) >> 24;
    match kind {
        ACT_COMMAND => match action {
            COMMAND_STAY => {
                ai::executor::execute_actions(
                    world,
                    pet_guid,
                    vec![AIAction::ClearAttackTarget, AIAction::StopMovement],
                );
                world.managers.creature_mgr.with_creature_mut(pet_guid, |c| {
                    c.motion_master.move_idle(pet_guid);
                    c.following_target = None;
                });
            }
            COMMAND_FOLLOW => {
                ai::executor::execute_actions(world, pet_guid, vec![AIAction::PetFollowOwner]);
            }
            COMMAND_ATTACK => {
                if target.is_empty() || target == player_guid {
                    return Ok(());
                }
                ai::queue_event(world, pet_guid, AIEvent::Engage { target_guid: target });
            }
            COMMAND_ABANDON => {
                world.systems.pet.dismiss(player_guid, world);
            }
            _ => {}
        },
        ACT_REACTION => {
            let react_state = match action {
                REACT_PASSIVE => ReactState::Passive,
                REACT_DEFENSIVE => ReactState::Defensive,
                _ => ReactState::Aggressive,
            };
            let mut actions = vec![AIAction::SetReactState { react_state }];
            if react_state == ReactState::Passive {
                actions.push(AIAction::PetFollowOwner);
            }
            ai::executor::execute_actions(world, pet_guid, actions);
        }
        _ => {
            // A pet spell from the action bar.
            if world.managers.spell_mgr.get(action).is_none() {
                return Ok(());
            }
            let spell_target = if target.is_empty() { None } else { Some(target) };
            world
                .systems
                .spells
                .cast_spell(pet_guid, action, spell_target, false, world)
                .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::Opcode;

    #[test]
    fn vanilla_pet_action_reads_guid_word_and_target() {
        let pet = ObjectGuid::new_creature(1, 5);
        let target = ObjectGuid::new_creature(2, 6);
        let mut packet = WorldPacket::new(Opcode::CMSG_PET_ACTION);
        packet.write_u64(pet.raw());
        packet.write_u32((ACT_COMMAND << 24) | COMMAND_ATTACK);
        packet.write_u64(target.raw());
        let (read_pet, data, read_target) = read_pet_action(Protocol::Vanilla, &mut packet).unwrap();
        assert_eq!(read_pet, pet);
        assert_eq!(data >> 24, ACT_COMMAND);
        assert_eq!(data & 0xFF_FFFF, COMMAND_ATTACK);
        assert_eq!(read_target, target);
    }
}
