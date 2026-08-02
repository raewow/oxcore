//! Gossip packet handlers
//!
//! Slim handlers that parse packets and delegate to GossipSystem.

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::core::common::packet::WorldPacketGuidExt;
use crate::core::lua::{build_player_snapshot, execute_gossip_actions};
use crate::core::session::WorldSession;
use crate::game::common::creature_flags::CREATURE_STATIC_FLAG_VISIBLE_TO_GHOSTS;
use crate::game::common::player_constants::get_faction_for_race;
use crate::game::creature::ai::is_hostile_faction;
use crate::game::player::spells::state::CurrentSpellType;
use crate::World;
use oxcore_shared::protocol::bitbuf::BitReader;
use oxcore_shared::protocol::{ObjectGuid, Opcode, Protocol, WorldPacket};

const NPC_FLAG_BANKER: u32 = 0x00000100;
const INTERACTION_DISTANCE: f32 = 5.0;
const UNIT_FLAG_NOT_SELECTABLE: u32 = 0x02000000;

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
) -> Result<(ObjectGuid, u32)> {
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
        reader
            .read_string(code_length)
            .ok_or_else(|| anyhow::anyhow!("Failed to read promotion code"))?;
        Ok((ObjectGuid::from_guid128(high, low), option_id))
    } else {
        let npc_guid = packet
            .read_guid()
            .ok_or_else(|| anyhow::anyhow!("Failed to read NPC GUID"))?;
        let option_id = packet
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read option ID"))?;
        let _coded_text = packet.read_cstring();
        Ok((npc_guid, option_id))
    }
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

    info!(
        "CMSG_GOSSIP_HELLO: player={:?}, npc={:?}",
        player_guid, npc_guid
    );

    // Get creature info to check if it's a quest giver
    let (entry, npc_flags, creature_type) = {
        let Some(creature) = world.managers.creature_mgr.get_creature(npc_guid) else {
            debug!(
                "Gossip hello for unresolved or non-creature guid {:?}",
                npc_guid
            );
            return Ok(());
        };

        if !creature.is_alive() {
            debug!("Gossip hello for dead creature {:?}", npc_guid);
            return Ok(());
        }

        let can_interact = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
                if player.map_id != creature.map_id {
                    return false;
                }

                if player.is_alive()
                    && (creature.static_flags1 & CREATURE_STATIC_FLAG_VISIBLE_TO_GHOSTS) != 0
                {
                    return false;
                }

                if !player.is_alive()
                    && (creature.static_flags1 & CREATURE_STATIC_FLAG_VISIBLE_TO_GHOSTS) == 0
                {
                    return false;
                }

                if is_hostile_faction(creature.faction, get_faction_for_race(player.race), true) {
                    return false;
                }

                if creature.unit_flags & UNIT_FLAG_NOT_SELECTABLE != 0 {
                    return false;
                }

                player
                    .movement
                    .position
                    .is_within_range(&creature.position, INTERACTION_DISTANCE)
            })
            .unwrap_or(false);

        if !can_interact || world.managers.creature_mgr.is_in_combat(npc_guid) {
            debug!(
                "Player {:?} cannot interact with {:?}",
                player_guid, npc_guid
            );
            return Ok(());
        }

        (creature.entry, creature.npc_flags, creature.creature_type)
    };

    {
        const CREATURE_FLAG_EXTRA_CIVILIAN: u32 = 0x00000002;
        const CREATURE_TYPE_TOTEM: u8 = 11;

        let is_civilian = world
            .managers
            .creature_mgr
            .get_template(entry)
            .map(|t| (t.flags_extra & CREATURE_FLAG_EXTRA_CIVILIAN) != 0)
            .unwrap_or(false);
        let is_totem = creature_type == CREATURE_TYPE_TOTEM;

        if !is_civilian && !is_totem {
            world
                .managers
                .creature_mgr
                .with_creature_mut(npc_guid, |c| {
                    c.pause_out_of_combat_movement();
                });
        }
    }

    const INTERACT_INTERRUPT_FLAGS: u32 = 0x00000C00;

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

    info!(
        "NPC entry={}, npc_flags=0x{:08X}, has_gossip={}, has_quest={}, has_vendor={}",
        entry,
        npc_flags,
        (npc_flags & 0x00000001) != 0,
        (npc_flags & 0x00000002) != 0,
        (npc_flags & 0x00000080) != 0
    );

    // Spirit guides send the battleground resurrection-wave timer when greeted.
    const NPC_FLAG_SPIRITGUIDE: u32 = 0x00000040;
    if (npc_flags & NPC_FLAG_SPIRITGUIDE) != 0 {
        let mut reply = WorldPacket::new(Opcode::SMSG_AREA_SPIRIT_HEALER_TIME);
        reply.write_u64(npc_guid.raw());
        reply.write_u32(0);
        session.send_packet(reply)?;
    }

    // Spirit healer: NPC_FLAG_SPIRITHEALER = 0x20. When a dead ghost player
    // clicks a spirit healer, skip gossip and send SMSG_SPIRIT_HEALER_CONFIRM
    // directly. The client then shows the "Do you wish to be resurrected?" dialog
    // and responds with CMSG_SPIRIT_HEALER_ACTIVATE on confirmation.
    const NPC_FLAG_SPIRITHEALER: u32 = 0x00000020;
    if (npc_flags & NPC_FLAG_SPIRITHEALER) != 0 {
        let is_ghost = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
                use crate::game::player::death::DeathState;
                player.death.death_state == DeathState::Dead
            })
            .unwrap_or(false);

        if is_ghost {
            info!(
                "Spirit healer interaction: player {:?} (ghost) -> NPC {:?}, sending SMSG_SPIRIT_HEALER_CONFIRM",
                player_guid, npc_guid
            );
            let mut confirm = WorldPacket::new(Opcode::SMSG_SPIRIT_HEALER_CONFIRM);
            // Payload: the spirit healer's full GUID (8 bytes)
            confirm.write_u64(npc_guid.raw());
            session.send_packet(confirm)?;
            return Ok(());
        }
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

    // Prepare quest data if NPC is a quest giver
    let quest_data = if (npc_flags & 0x00000002) != 0 {
        // QUESTGIVER flag
        info!("NPC has QUESTGIVER flag, preparing quest menu");
        let quests = world
            .systems
            .quest
            .prepare_quest_menu(player_guid, entry, world);
        info!("Prepared {} quests for menu", quests.len());
        for quest in &quests {
            info!(
                "  Quest {}: '{}' (icon={}, level={})",
                quest.quest_id, quest.title, quest.icon, quest.level
            );
        }
        Some(quests)
    } else {
        info!("NPC does not have QUESTGIVER flag");
        None
    };

    // Check if we should auto-display a single quest.
    // Conditions: Has exactly 1 quest, is not a vendor, and has no usable gossip menu.
    let has_gossip_flag = (npc_flags & 0x00000001) != 0;
    let has_vendor_flag = (npc_flags & 0x00000080) != 0;
    let has_gossip_menu = world.systems.gossip.has_gossip_menu(entry);
    let quest_count = quest_data.as_ref().map(|q| q.len()).unwrap_or(0);

    // Determine if we should auto-display the quest
    let should_auto_display =
        quest_count == 1 && !has_vendor_flag && (!has_gossip_flag || !has_gossip_menu);

    if should_auto_display {
        // Auto-display the single quest directly
        if let Some(ref quests) = quest_data {
            if let Some(quest) = quests.first() {
                let quest_id = quest.quest_id;
                info!(
                    "Auto-displaying single quest {} for player {:?} from NPC {:?}",
                    quest_id, player_guid, npc_guid
                );

                // Check quest status to determine which dialog to show
                use crate::game::npc::quest::types::QuestStatus;
                let quest_status = world.systems.quest.get_quest_status(player_guid, quest_id);

                match quest_status {
                    QuestStatus::Complete => {
                        // Quest is complete - show reward dialog
                        info!(
                            "Quest {} is complete for player {:?}, showing reward dialog",
                            quest_id, player_guid
                        );
                        world
                            .systems
                            .quest
                            .handle_quest_complete(player_guid, npc_guid, quest_id, world)
                            .await?;
                    }
                    QuestStatus::Incomplete => {
                        // Quest is incomplete - show request items/objectives dialog
                        info!(
                            "Quest {} is incomplete for player {:?}",
                            quest_id, player_guid
                        );
                        world
                            .systems
                            .quest
                            .handle_quest_complete(player_guid, npc_guid, quest_id, world)
                            .await?;
                    }
                    _ => {
                        // Quest is available or none - show quest details for accepting
                        info!(
                            "Quest {} is available for player {:?}, showing details dialog",
                            quest_id, player_guid
                        );
                        world.systems.quest.send_quest_details(
                            player_guid,
                            npc_guid,
                            quest_id,
                            world,
                        )?;
                    }
                }
                return Ok(());
            }
        }
    }

    // Check if vendor should open directly (no gossip menu)
    // Conditions: NPC is a vendor, has no GOSSIP flag, and has no gossip menu defined
    if has_vendor_flag && !has_gossip_flag {
        // Check if vendor has a gossip menu defined
        if !has_gossip_menu {
            // Open vendor window directly
            info!(
                "Opening vendor window directly for player {:?} from NPC {:?} (no gossip menu)",
                player_guid, npc_guid
            );
            world
                .systems
                .vendor
                .send_vendor_list(player_guid, npc_guid)
                .await?;
            return Ok(());
        }
    }

    // Check if trainer should open directly (no gossip menu)
    let has_trainer_flag = (npc_flags & 0x00000010) != 0; // NPC_FLAG_TRAINER
    if has_trainer_flag && !has_gossip_flag {
        let has_gossip_menu = world.systems.gossip.has_gossip_menu(entry);
        if !has_gossip_menu {
            info!(
                "Opening trainer window directly for player {:?} from NPC {:?} (no gossip menu)",
                player_guid, npc_guid
            );
            crate::handlers::trainer_handler::send_trainer_list(player_guid, npc_guid, world)
                .await?;
            return Ok(());
        }
    }

    // Convert quest data to gossip format
    let gossip_quests = quest_data.map(|quests| {
        quests
            .into_iter()
            .map(|q| oxcore_shared::messages::GossipQuestData {
                quest_id: q.quest_id,
                icon: q.icon,
                level: q.level,
                title: q.title,
            })
            .collect()
    });

    // Delegate to gossip system
    world
        .systems
        .gossip
        .send_gossip_menu(player_guid, npc_guid, None, gossip_quests)
        .await?;

    Ok(())
}

/// Handle CMSG_GOSSIP_SELECT_OPTION (0x17C)
///
/// Sent when player selects a gossip option.
/// Packet format: GUID (packed), option_id (u32), [code (cstring)]
pub async fn handle_gossip_select_option(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // Modern body adds GossipID and bit-length-prefixes the promotion code.
    let (npc_guid, option_id) = read_gossip_select_option(session.protocol(), packet)?;

    debug!(
        "CMSG_GOSSIP_SELECT_OPTION: player={:?}, npc={:?}, option={}",
        player_guid, npc_guid, option_id
    );

    // Check for a Lua gossip script that handles OnGossipSelect
    let npc_entry = world
        .managers
        .creature_mgr
        .get_creature(npc_guid)
        .map(|c| c.entry)
        .unwrap_or(0);
    if npc_entry > 0 {
        if let Some(script) = world.managers.lua_mgr.get_gossip_script(npc_entry) {
            // Get the current menu_id to pass to the script
            let menu_id = world
                .managers
                .player_mgr
                .get_player(player_guid)
                .and_then(|p| p.current_gossip_menu_id)
                .unwrap_or(0);
            let player_snap = build_player_snapshot(player_guid, world);
            let actions = world.managers.lua_mgr.with_lua(|lua| {
                script.on_gossip_select(lua, &player_snap, npc_guid, menu_id, option_id)
            });
            if !actions.is_empty() {
                execute_gossip_actions(actions, player_guid, npc_guid, world).await?;
                return Ok(());
            }
        }
    }

    // Get the current menu_id for this player from the player state
    // TODO: Track current gossip menu per player
    let menu_id = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .and_then(|p| p.current_gossip_menu_id)
        .unwrap_or(0);

    // Get the selected option details before handling
    let option_details = world.systems.gossip.get_option_details(menu_id, option_id);

    // Delegate to gossip system
    world
        .systems
        .gossip
        .handle_option_select(player_guid, npc_guid, menu_id, option_id)
        .await?;

    // Handle special cases after gossip system closes the window
    if let Some(option) = option_details {
        use crate::game::npc::gossip::types::gossip_option;
        match option.option_id {
            gossip_option::VENDOR | gossip_option::ARMORER => {
                // Open vendor window
                info!(
                    "Opening vendor window for player {:?} from NPC {:?}",
                    player_guid, npc_guid
                );
                world
                    .systems
                    .vendor
                    .send_vendor_list(player_guid, npc_guid)
                    .await?;
            }
            gossip_option::TRAINER => {
                // Open trainer window
                info!(
                    "Opening trainer window for player {:?} from NPC {:?}",
                    player_guid, npc_guid
                );
                crate::handlers::trainer_handler::send_trainer_list(player_guid, npc_guid, world)
                    .await?;
            }
            _ => {
                // Other options are handled by the gossip system
            }
        }
    }

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
                probability: opt.probability,
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
