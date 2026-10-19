//! Gossip packet handlers
//!
//! Slim handlers that parse packets and delegate to GossipSystem.

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::core::common::packet::WorldPacketGuidExt;
use crate::core::lua::{build_player_snapshot, execute_gossip_actions};
use crate::core::session::WorldSession;
use crate::game::npc::interaction::{can_interact_with_gameobject, can_interact_with_npc};
use crate::game::player::spells::state::CurrentSpellType;
use crate::World;
use oxcore_shared::protocol::bitbuf::BitReader;
use oxcore_shared::protocol::{ObjectGuid, Opcode, Protocol, WorldPacket};

const NPC_FLAG_BANKER: u32 = 0x00000100;

fn read_gossip_hello_guid(protocol: Protocol, packet: &mut WorldPacket) -> Result<ObjectGuid> {
    packet
        .read_guid_for(protocol)
        .ok_or_else(|| anyhow::anyhow!("Failed to read NPC GUID"))
}

fn read_npc_text_query(protocol: Protocol, packet: &mut WorldPacket) -> Result<u32> {
    if protocol == Protocol::Modern {
        let mut reader = BitReader::new(packet.contents());
        let text_id = reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read text ID"))?;
        reader
            .read_packed_guid_128()
            .ok_or_else(|| anyhow::anyhow!("Failed to read modern NPC GUID"))?;
        Ok(text_id)
    } else {
        let text_id = packet
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read text ID"))?;
        packet
            .read_guid()
            .ok_or_else(|| anyhow::anyhow!("Failed to read NPC GUID"))?;
        Ok(text_id)
    }
}

fn read_gossip_select_option(
    protocol: Protocol,
    packet: &mut WorldPacket,
) -> Result<(ObjectGuid, u32, String)> {
    if protocol == Protocol::Modern {
        let mut reader = BitReader::new(packet.contents());
        let (high, low) = reader
            .read_packed_guid_128()
            .ok_or_else(|| anyhow::anyhow!("Failed to read modern NPC GUID"))?;
        let _gossip_id = reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read gossip ID"))?;
        let option_id = reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read option ID"))?;
        let code_length = reader
            .read_bits(8)
            .ok_or_else(|| anyhow::anyhow!("Failed to read promotion code length"))?
            as usize;
        let code = reader
            .read_string(code_length)
            .ok_or_else(|| anyhow::anyhow!("Failed to read promotion code"))?;
        Ok((ObjectGuid::from_guid128(high, low), option_id, code))
    } else {
        let npc_guid = packet
            .read_guid()
            .ok_or_else(|| anyhow::anyhow!("Failed to read NPC GUID"))?;
        let option_id = packet
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read option ID"))?;
        let code = packet.read_cstring().unwrap_or_default();
        Ok((npc_guid, option_id, code))
    }
}

/// Talking to an NPC or using a menu: breaks interact-cancelled auras and channels, and
/// stops a wandering creature so the player can talk to it.
pub(crate) async fn begin_npc_interaction(
    world: &World,
    player_guid: ObjectGuid,
    source_guid: ObjectGuid,
) -> Result<()> {
    const INTERACT_INTERRUPT_FLAGS: u32 = 0x0000_0C00; // TALK (0x400) | USE (0x800)

    world
        .systems
        .auras
        .remove_auras_with_interrupt_flag(player_guid, INTERACT_INTERRUPT_FLAGS, world)
        .await?;

    let should_interrupt_channel = world
        .systems
        .player
        .manager()
        .with_player(player_guid, |player| {
            player
                .spells
                .get_current_spell(CurrentSpellType::Channeled)
                .and_then(|cast| {
                    world.managers.spell_mgr.get(cast.spell_id).filter(|entry| {
                        (entry.channel_interrupt_flags & INTERACT_INTERRUPT_FLAGS) != 0
                    })
                })
                .is_some()
        })
        .unwrap_or(false);
    if should_interrupt_channel {
        world.systems.spells.cancel_cast(player_guid, world).await?;
    }

    if source_guid.is_creature_or_pet() {
        const CREATURE_FLAG_EXTRA_CIVILIAN: u32 = 0x0000_0002;
        const CREATURE_TYPE_TOTEM: u8 = 11;

        let Some((entry, creature_type)) = world
            .managers
            .creature_mgr
            .with_creature(source_guid, |c| (c.entry, c.creature_type))
        else {
            return Ok(());
        };
        let is_civilian = world
            .managers
            .creature_mgr
            .get_template(entry)
            .is_some_and(|t| (t.flags_extra & CREATURE_FLAG_EXTRA_CIVILIAN) != 0);
        if !is_civilian && creature_type != CREATURE_TYPE_TOTEM {
            world
                .managers
                .creature_mgr
                .with_creature_mut(source_guid, |c| c.pause_out_of_combat_movement());
        }
    }
    Ok(())
}

/// Handle CMSG_GOSSIP_HELLO (0x17B)
///
/// Sent when player right-clicks an NPC to open gossip.
/// Packet format: GUID (packed)
pub async fn handle_gossip_hello(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let npc_guid = read_gossip_hello_guid(session.protocol(), packet)?;

    debug!(
        "CMSG_GOSSIP_HELLO: player={:?}, npc={:?}",
        player_guid, npc_guid
    );

    if !can_interact_with_npc(world, player_guid, npc_guid, 0) {
        debug!(
            "Player {:?} cannot interact with {:?}",
            player_guid, npc_guid
        );
        return Ok(());
    }
    let Some((entry, npc_flags)) = world
        .managers
        .creature_mgr
        .with_creature(npc_guid, |c| (c.entry, c.npc_flags))
    else {
        return Ok(());
    };

    begin_npc_interaction(world, player_guid, npc_guid).await?;

    // Spirit guides send the battleground resurrection-wave timer when greeted.
    const NPC_FLAG_SPIRITGUIDE: u32 = 0x00000040;
    if (npc_flags & NPC_FLAG_SPIRITGUIDE) != 0 {
        session.send_msg(oxcore_shared::messages::death::SmsgAreaSpiritHealerTime {
            healer_guid: npc_guid,
            time_left_ms: 0,
        })?;
    }

    // Check for a Lua gossip script that handles OnGossipHello
    if let Some(script) = world.managers.lua_mgr.get_gossip_script(entry) {
        let player_snap = build_player_snapshot(player_guid, world);
        let actions = world
            .managers
            .lua_mgr
            .with_lua(|lua| script.on_gossip_hello(lua, &player_snap, npc_guid));
        if !actions.is_empty() {
            execute_gossip_actions(actions, player_guid, npc_guid, world).await?;
            return Ok(());
        }
    }

    world
        .systems
        .gossip
        .gossip_hello(world, player_guid, npc_guid)
        .await
}

/// Handle CMSG_GOSSIP_SELECT_OPTION (0x17C)
///
/// Sent when player selects a gossip option.
/// Packet format: GUID (packed), option index (u32), [code (cstring)]
pub async fn handle_gossip_select_option(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // Modern body adds GossipID and bit-length-prefixes the promotion code.
    let (source_guid, list_index, code) = read_gossip_select_option(session.protocol(), packet)?;

    debug!(
        "CMSG_GOSSIP_SELECT_OPTION: player={:?}, source={:?}, option={}",
        player_guid, source_guid, list_index
    );

    let Some(option) = world
        .systems
        .gossip
        .shown_option(player_guid, source_guid, list_index)
    else {
        debug!(
            "Gossip option {} was never offered to {:?} by {:?}",
            list_index, player_guid, source_guid
        );
        return Ok(());
    };
    // A coded option needs the code the player typed.
    if option.coded && code.is_empty() {
        return Ok(());
    }

    let can_interact = if source_guid.is_game_object() {
        can_interact_with_gameobject(world, player_guid, source_guid)
    } else {
        can_interact_with_npc(world, player_guid, source_guid, 0)
    };
    if !can_interact {
        debug!(
            "Player {:?} cannot interact with {:?}",
            player_guid, source_guid
        );
        return Ok(());
    }

    begin_npc_interaction(world, player_guid, source_guid).await?;

    // Check for a Lua gossip script that handles OnGossipSelect
    let npc_entry = world
        .managers
        .creature_mgr
        .with_creature(source_guid, |c| c.entry)
        .unwrap_or(0);
    if npc_entry > 0 {
        if let Some(script) = world.managers.lua_mgr.get_gossip_script(npc_entry) {
            let player_snap = build_player_snapshot(player_guid, world);
            let code = option.coded.then_some(code.as_str());
            let actions = world.managers.lua_mgr.with_lua(|lua| {
                script.on_gossip_select(
                    lua,
                    &player_snap,
                    source_guid,
                    option.sender,
                    option.action,
                    code,
                )
            });
            if !actions.is_empty() {
                execute_gossip_actions(actions, player_guid, source_guid, world).await?;
                return Ok(());
            }
        }
    }

    world
        .systems
        .gossip
        .on_gossip_select(world, player_guid, source_guid, list_index)
        .await
}

/// Talking to a trainer, petitioner or tabard designer ends feign death.
fn remove_feign_death(world: &World, player_guid: ObjectGuid) {
    use crate::game::player::auras::effects::AURA_FEIGN_DEATH;
    world.managers.player_mgr.with_player_mut(player_guid, |player| {
        if !player
            .auras
            .container
            .remove_spell_auras(AURA_FEIGN_DEATH)
            .is_empty()
        {
            player.auras.needs_client_update = true;
            player.auras.needs_stat_recalc = true;
        }
    });
}

/// Handle MSG_TALENT_WIPE_CONFIRM (1.14: CMSG_CONFIRM_RESPEC_WIPE)
///
/// The player accepted a class trainer's offer to reset their talents.
/// Packet format: trainer GUID
pub async fn handle_talent_wipe_confirm(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    const NPC_FLAG_TRAINER: u32 = 0x0000_0010;
    const UNTALENT_VISUAL_SPELL: u32 = 14867;

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let trainer_guid = packet
        .read_guid_for(session.protocol())
        .ok_or_else(|| anyhow::anyhow!("Failed to read trainer GUID"))?;

    if !can_interact_with_npc(world, player_guid, trainer_guid, NPC_FLAG_TRAINER) {
        debug!(
            "Talent wipe: player {:?} cannot interact with trainer {:?}",
            player_guid, trainer_guid
        );
        return Ok(());
    }
    remove_feign_death(world, player_guid);

    let cost = world.systems.talents.reset_cost(player_guid, world);
    if !world
        .systems
        .talents
        .reset_talents(player_guid, false, world)
        .await?
    {
        // An empty trainer tells the player there is nothing to reset.
        session.send_msg(oxcore_shared::messages::MsgTalentWipeConfirm {
            trainer_guid: ObjectGuid::empty(),
            cost,
        })?;
        return Ok(());
    }

    crate::game::creature::ai::executor::try_to_cast(
        world,
        trainer_guid,
        Some(player_guid),
        UNTALENT_VISUAL_SPELL,
        crate::game::creature::ai::executor::cast_flags::CF_TRIGGERED,
        100,
    );
    Ok(())
}

/// Handle CMSG_PETITION_SHOWLIST (0x1BB)
///
/// Sent when the player talks to a guild master without a gossip menu.
pub async fn handle_petition_showlist(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    const NPC_FLAG_PETITIONER: u32 = 0x0000_0200;

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let npc_guid = packet
        .read_guid_for(session.protocol())
        .ok_or_else(|| anyhow::anyhow!("Failed to read petitioner GUID"))?;

    if !can_interact_with_npc(world, player_guid, npc_guid, NPC_FLAG_PETITIONER) {
        return Ok(());
    }
    remove_feign_death(world, player_guid);
    session.send_msg(oxcore_shared::messages::SmsgPetitionShowlist { npc_guid })?;
    Ok(())
}

/// Handle MSG_TABARDVENDOR_ACTIVATE (1.14: CMSG_TABARD_VENDOR_ACTIVATE)
///
/// Sent when the player talks to a tabard designer without a gossip menu.
pub async fn handle_tabard_vendor_activate(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    const NPC_FLAG_TABARDDESIGNER: u32 = 0x0000_0400;

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let designer_guid = packet
        .read_guid_for(session.protocol())
        .ok_or_else(|| anyhow::anyhow!("Failed to read tabard designer GUID"))?;

    if !can_interact_with_npc(world, player_guid, designer_guid, NPC_FLAG_TABARDDESIGNER) {
        return Ok(());
    }
    remove_feign_death(world, player_guid);
    session.send_msg(oxcore_shared::messages::MsgTabardVendorActivate { designer_guid })?;
    Ok(())
}

/// Handle CMSG_BANKER_ACTIVATE (0x1B5)
///
/// Sent when the client activates a banker directly.
/// Packet format: GUID (unpacked)
pub async fn handle_banker_activate(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let banker_guid = packet
        .read_guid_for(session.protocol())
        .ok_or_else(|| anyhow::anyhow!("Failed to read banker GUID"))?;

    let can_use_banker = world
        .managers
        .creature_mgr
        .get_creature(banker_guid)
        .map(|creature| (creature.npc_flags & NPC_FLAG_BANKER) != 0)
        .unwrap_or(false);

    if !can_use_banker {
        warn!(
            "CMSG_BANKER_ACTIVATE rejected: player={:?}, banker={:?}",
            player_guid, banker_guid
        );
        return Ok(());
    }

    info!(
        "CMSG_BANKER_ACTIVATE: player={:?}, banker={:?}",
        player_guid, banker_guid
    );
    if let Some(mut player) = world.managers.player_mgr.get_player_mut(player_guid) {
        player.current_banker_guid = Some(banker_guid);
    }
    session.send_msg(oxcore_shared::messages::SmsgShowBank { banker_guid })?;

    Ok(())
}

/// Handle CMSG_NPC_TEXT_QUERY (0x17F)
///
/// Sent when client requests NPC text data.
/// Packet format: text_id (u32), GUID (packed)
pub async fn handle_npc_text_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let text_id = read_npc_text_query(session.protocol(), packet)?;

    debug!("CMSG_NPC_TEXT_QUERY: text_id={}", text_id);

    use oxcore_shared::messages::gossip::{NpcTextOption, SmsgNpcTextUpdate};

    // Look up NPC text from gossip manager
    let msg = if let Some(npc_text) = world.systems.gossip_manager.get_npc_text(text_id) {
        let options = npc_text.options.map(|opt| {
            let bct = world
                .systems
                .gossip_manager
                .get_broadcast_text(opt.broadcast_text_id);
            NpcTextOption {
                // A line without text is never picked.
                probability: if bct.is_some() { opt.probability } else { 0.0 },
                broadcast_text_id: opt.broadcast_text_id,
                male_text: bct
                    .as_ref()
                    .map(|b| b.male_text.clone())
                    .unwrap_or_default(),
                female_text: bct
                    .as_ref()
                    .map(|b| b.female_text.clone())
                    .unwrap_or_default(),
                language_id: bct.as_ref().map(|b| b.language_id).unwrap_or(0),
                emote_delays: bct.as_ref().map(|b| b.emote_delays).unwrap_or([0; 3]),
                emote_ids: bct.as_ref().map(|b| b.emote_ids).unwrap_or([0; 3]),
            }
        });
        SmsgNpcTextUpdate { text_id, options }
    } else {
        // Text not found - send default empty response
        let options = std::array::from_fn(|_| NpcTextOption::default());
        SmsgNpcTextUpdate { text_id, options }
    };

    world
        .managers
        .broadcast_mgr
        .send_msg_to_player(player_guid, msg);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::bitbuf::BitWriter;

    #[test]
    fn modern_npc_text_query_reads_packed_guid128() {
        let mut writer = BitWriter::new();
        writer.write_u32(42);
        writer.write_packed_guid_128(0x0100, 0x0200_01);
        let mut packet = WorldPacket::new(Opcode::CMSG_NPC_TEXT_QUERY);
        packet.write_bytes(&writer.into_bytes());

        assert_eq!(
            read_npc_text_query(Protocol::Modern, &mut packet).unwrap(),
            42
        );
    }

    #[test]
    fn modern_gossip_hello_reads_packed_guid128() {
        let expected = ObjectGuid::new_creature(197, 42);
        let (high, low) = expected.to_guid128(1);
        let mut writer = BitWriter::new();
        writer.write_packed_guid_128(high, low);
        let mut packet = WorldPacket::new(Opcode::CMSG_GOSSIP_HELLO);
        packet.write_bytes(&writer.into_bytes());

        assert_eq!(
            read_gossip_hello_guid(Protocol::Modern, &mut packet).unwrap(),
            expected
        );
    }

    #[test]
    fn modern_gossip_select_reads_index_after_gossip_id() {
        let mut writer = BitWriter::new();
        writer.write_packed_guid_128(0x0100, 0x0200_01);
        writer.write_u32(7); // GossipID
        writer.write_u32(3); // GossipIndex
        writer.write_bits(4, 8);
        writer.write_string_raw("code");
        let mut packet = WorldPacket::new(Opcode::CMSG_GOSSIP_SELECT_OPTION);
        packet.write_bytes(&writer.into_bytes());

        assert_eq!(
            read_gossip_select_option(Protocol::Modern, &mut packet)
                .unwrap()
                .1,
            3
        );
    }
}
