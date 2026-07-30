//! Quest packet handlers
//!
//! Slim handlers that parse packets and delegate to QuestSystem.

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::core::common::packet::WorldPacketGuidExt;
use crate::core::lua::{build_player_snapshot, execute_gossip_actions};
use crate::core::session::WorldSession;
use crate::game::common::player_constants::get_faction_for_race;
use crate::game::creature::ai::{
    is_hostile_faction, is_npc, NPC_FLAG_GOSSIP, NPC_FLAG_QUEST_GIVER, NPC_FLAG_VENDOR,
};
use crate::game::npc::quest::system::QUEST_SHARE_DISTANCE;
use crate::game::npc::quest::types::{QuestStatus, MAX_QUEST_LOG_SIZE};
use crate::game::player::auras::effects::AURA_FEIGN_DEATH;
use crate::game::player::player::QuestShareInfo;
use crate::game::player::spells::state::CurrentSpellType;
use crate::World;
use oxcore_shared::messages::gossip::SmsgGossipComplete;
use oxcore_shared::messages::quest::{
    MsgQuestPushResult, QuestObjectiveData, SmsgQuestQueryResponseV2,
};
use oxcore_shared::protocol::{ObjectGuid, Opcode, Position, WorldPacket};

/// Handle CMSG_QUESTGIVER_STATUS_QUERY (0x182)
///
/// Sent when player approaches an NPC to check quest status.
/// Packet format: GUID (packed)
pub async fn handle_questgiver_status_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let quest_giver_guid = read_questgiver_guid(session.protocol(), packet)?;

    debug!(
        "CMSG_QUESTGIVER_STATUS_QUERY: player={:?}, npc={:?}",
        player_guid, quest_giver_guid
    );

    // Hostile creature questgivers always show no quest status
    if quest_giver_guid.is_creature() {
        let is_hostile = world
            .managers
            .creature_mgr
            .get_creature(quest_giver_guid)
            .and_then(|c| {
                world.managers.player_mgr.with_player(player_guid, |p| {
                    is_hostile_faction(c.faction, get_faction_for_race(p.race), true)
                })
            })
            .unwrap_or(false);
        if is_hostile {
            debug!(
                "Questgiver {:?} is hostile to player {:?}, suppressing status",
                quest_giver_guid, player_guid
            );
            let msg = oxcore_shared::messages::quest::SmsgQuestgiverStatus {
                guid: quest_giver_guid,
                status: oxcore_shared::messages::quest::DialogStatus::None,
            };
            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(player_guid, msg);
            return Ok(());
        }
    }

    // Delegate to quest system for script/relation-based status
    world
        .systems
        .quest
        .send_quest_giver_status(player_guid, quest_giver_guid, world);

    Ok(())
}

/// Read a quest giver's GUID in whichever form the client speaks.
///
/// Thin wrapper over [`WorldPacketGuidExt::read_guid_for`] so every quest handler shares one error
/// message. The 128-bit decode itself lives in `ObjectGuid::from_guid128`, next to its inverse, so
/// the two directions are round-trip tested together.
fn read_questgiver_guid(
    protocol: oxcore_shared::protocol::Protocol,
    packet: &mut WorldPacket,
) -> Result<ObjectGuid> {
    packet
        .read_guid_for(protocol)
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest giver GUID"))
}

/// Handle CMSG_QUEST_QUERY (0x5C)
///
/// Sends quest metadata to the client.
pub async fn handle_quest_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let quest_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest ID"))?;

    debug!("CMSG_QUEST_QUERY: quest={}", quest_id);

    let Some(quest) = world.systems.quest.manager.get_quest_template(quest_id) else {
        return Ok(());
    };

    let objectives_data = std::array::from_fn(|idx| QuestObjectiveData {
        creature_or_go_id: quest.req_creature_or_go_id[idx],
        creature_or_go_count: quest.req_creature_or_go_count[idx],
        item_id: quest.req_item_id[idx],
        item_count: quest.req_item_count[idx],
    });

    let msg = SmsgQuestQueryResponseV2 {
        quest_id: quest.id,
        method: quest.method as u32,
        quest_level: quest.quest_level,
        min_level: quest.min_level,
        zone_or_sort: quest.zone_or_sort,
        quest_type: quest.quest_type as u32,
        rep_objective_faction: quest.rep_objective_faction,
        rep_objective_value: quest.rep_objective_value,
        next_quest_in_chain: quest.next_quest_in_chain,
        rew_or_req_money: quest.rew_or_req_money,
        rew_money_max_level: quest.rew_money_max_level,
        rew_spell: quest.rew_spell,
        rew_spell_cast: quest.rew_spell_cast,
        src_item_id: quest.src_item_id,
        quest_flags: oxcore_shared::messages::quest::QuestFlags(quest.quest_flags.bits()),
        rew_item_id: quest.rew_item_id,
        rew_item_count: quest.rew_item_count,
        rew_choice_item_id: quest.rew_choice_item_id,
        rew_choice_item_count: quest.rew_choice_item_count,
        point_map_id: quest.point_map_id,
        point_x: quest.point_x,
        point_y: quest.point_y,
        point_opt: quest.point_opt,
        suggested_players: quest.suggested_players,
        limit_time: quest.limit_time,
        rew_rep_faction: quest.rew_rep_faction,
        rew_rep_value: quest.rew_rep_value,
        title: &quest.title,
        objectives: &quest.objectives,
        details: &quest.details,
        end_text: &quest.end_text,
        objectives_data,
        objective_text: &quest.objective_text,
    };

    session.send_msg(msg)?;
    Ok(())
}

/// Handle CMSG_QUESTGIVER_HELLO (0x184)
///
/// Sent when player right-clicks a quest giver NPC.
/// Packet format: GUID (packed)
pub async fn handle_questgiver_hello(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let quest_giver_guid = read_questgiver_guid(session.protocol(), packet)?;

    info!(
        "CMSG_QUESTGIVER_HELLO: player={:?}, npc={:?}",
        player_guid, quest_giver_guid
    );

    // Resolve creature questgiver with interaction validation.
    // Must exist, be alive, and have NPC interaction flags (gossip or quest giver).
    let (entry, npc_flags, creature_type) = {
        let Some(creature) = world.managers.creature_mgr.get_creature(quest_giver_guid) else {
            debug!(
                "Questgiver hello for unresolved or non-creature guid {:?}",
                quest_giver_guid
            );
            return Ok(());
        };

        if !creature.is_alive() {
            debug!("Questgiver {:?} is dead, rejecting hello", quest_giver_guid);
            return Ok(());
        }

        let npc_flags = creature.npc_flags;

        // Must have at least one NPC interaction flag to open quest/gossip
        if !is_npc(npc_flags) {
            debug!(
                "Questgiver {:?} has no interaction flags (0x{:08X}), rejecting hello",
                quest_giver_guid, npc_flags
            );
            return Ok(());
        }

        (creature.entry, npc_flags, creature.creature_type)
    };

    // Feign death cleanup (matches auction handler pattern)
    world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            let removed = player.auras.container.remove_spell_auras(AURA_FEIGN_DEATH);
            if !removed.is_empty() {
                player.auras.needs_client_update = true;
                player.auras.needs_stat_recalc = true;
                debug!(
                    "Cleared {} feign death aura(s) for player {:?}",
                    removed.len(),
                    player_guid
                );
            }
        });

    // Movement pause for non-civilian, non-totem creatures
    // (matches vmangos Creature::OnPlayerInteract behaviour)
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
                .with_creature_mut(quest_giver_guid, |c| {
                    c.pause_out_of_combat_movement();
                });
        }
    }

    // --- Interacting spells/auras cleanup ---
    // Matches vmangos HandleQuestgiverHelloOpcode:
    //   pPlayer->RemoveAurasWithInterruptFlags(AURA_INTERRUPT_FLAG_QUEST | AURA_INTERRUPT_FLAG_SPEECH)
    //   pPlayer->InterruptSpellsWithChannelFlags(AURA_INTERRUPT_FLAG_QUEST | AURA_INTERRUPT_FLAG_SPEECH)
    const INTERACT_INTERRUPT_FLAGS: u32 = 0x00000C00; // TALK (0x400) | USE (0x800)

    world
        .systems
        .auras
        .remove_auras_with_interrupt_flag(player_guid, INTERACT_INTERRUPT_FLAGS, world)
        .await?;

    {
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
    }

    // Lua OnGossipHello first chance
    if let Some(script) = world.managers.lua_mgr.get_gossip_script(entry) {
        let player_snap = build_player_snapshot(player_guid, world);
        let actions = world
            .managers
            .lua_mgr
            .with_lua(|lua| script.on_gossip_hello(lua, &player_snap, quest_giver_guid));
        if !actions.is_empty() {
            execute_gossip_actions(actions, player_guid, quest_giver_guid, world).await?;
            return Ok(());
        }
    }

    // Build the quest list this NPC offers/accepts for the player.
    let quest_list = if (npc_flags & NPC_FLAG_QUEST_GIVER) != 0 {
        world
            .systems
            .quest
            .prepare_quest_menu(player_guid, entry, world)
    } else {
        Vec::new()
    };

    // SendPreparedGossip: an NPC without a usable gossip menu that has quests
    // (and is not a vendor) opens the quest interface directly.
    let has_gossip_flag = (npc_flags & NPC_FLAG_GOSSIP) != 0;
    let has_vendor_flag = (npc_flags & NPC_FLAG_VENDOR) != 0;
    let has_gossip_menu = world.systems.gossip.has_gossip_menu(entry);

    if (!has_gossip_flag || !has_gossip_menu) && !has_vendor_flag && !quest_list.is_empty() {
        if quest_list.len() == 1 {
            let quest_id = quest_list[0].quest_id;
            match world.systems.quest.get_quest_status(player_guid, quest_id) {
                QuestStatus::Complete | QuestStatus::Incomplete => {
                    world
                        .systems
                        .quest
                        .handle_quest_complete(player_guid, quest_giver_guid, quest_id, world)
                        .await?;
                }
                _ => {
                    world.systems.quest.send_quest_details(
                        player_guid,
                        quest_giver_guid,
                        quest_id,
                        world,
                    )?;
                }
            }
        } else {
            world.systems.quest.send_quest_giver_quest_list(
                player_guid,
                quest_giver_guid,
                entry,
                world,
            )?;
        }
        return Ok(());
    }

    // Otherwise show the gossip menu (any quests are folded into its quest section).
    let quest_data = if quest_list.is_empty() {
        None
    } else {
        Some(
            quest_list
                .into_iter()
                .map(|q| oxcore_shared::messages::GossipQuestData {
                    quest_id: q.quest_id,
                    icon: q.icon,
                    level: q.level,
                    title: q.title,
                })
                .collect(),
        )
    };

    world
        .systems
        .gossip
        .send_gossip_menu(player_guid, quest_giver_guid, None, quest_data)
        .await?;

    Ok(())
}

/// Handle CMSG_QUESTGIVER_QUERY_QUEST (0x186)
///
/// Sent when player clicks on a quest in the quest giver list.
/// Packet format: GUID (packed), quest_id (u32)
pub async fn handle_questgiver_query_quest(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let quest_giver_guid = read_questgiver_guid(session.protocol(), packet)?;

    let quest_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest ID"))?;

    info!(
        "CMSG_QUESTGIVER_QUERY_QUEST: player={:?}, npc={:?}, quest={}",
        player_guid, quest_giver_guid, quest_id
    );

    let related =
        world
            .systems
            .quest
            .quest_giver_can_start_or_finish(quest_giver_guid, quest_id, world)
            || world
                .systems
                .inventory
                .cache()
                .get_item(player_guid, quest_giver_guid)
                .and_then(|item| {
                    world
                        .managers
                        .item_mgr
                        .get_template(item.read().entry)
                        .map(|template| template.start_quest == quest_id)
                })
                .unwrap_or(false);

    if !related {
        world
            .managers
            .broadcast_mgr
            .send_msg_to_player(player_guid, SmsgGossipComplete);
        return Ok(());
    }

    if let Some(quest) = world.systems.quest.manager.get_quest_template(quest_id) {
        info!(
            "Quest {} is related to {:?}, showing details dialog",
            quest.id, player_guid
        );
        world
            .systems
            .quest
            .send_quest_details(player_guid, quest_giver_guid, quest.id, world)?;
    }

    Ok(())
}

/// Handle CMSG_QUESTGIVER_ACCEPT_QUEST (0x189)
///
/// Sent when player clicks "Accept" on a quest.
/// Packet format: GUID (packed), quest_id (u32)
pub async fn handle_questgiver_accept_quest(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let quest_giver_guid = read_questgiver_guid(session.protocol(), packet)?;

    let quest_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest ID"))?;

    info!(
        "CMSG_QUESTGIVER_ACCEPT_QUEST: player={:?}, npc={:?}, quest={}",
        player_guid, quest_giver_guid, quest_id
    );

    // Delegate to quest system
    world
        .systems
        .quest
        .handle_quest_accept(player_guid, quest_giver_guid, quest_id, world)
        .await?;

    Ok(())
}

/// Handle CMSG_QUESTGIVER_COMPLETE_QUEST (0x18E)
///
/// Sent when player clicks "Complete Quest" on a quest giver.
/// Packet format: GUID (packed), quest_id (u32)
pub async fn handle_questgiver_complete_quest(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let quest_giver_guid = read_questgiver_guid(session.protocol(), packet)?;

    let quest_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest ID"))?;

    info!(
        "CMSG_QUESTGIVER_COMPLETE_QUEST: player={:?}, npc={:?}, quest={}",
        player_guid, quest_giver_guid, quest_id
    );

    // Delegate to quest system
    world
        .systems
        .quest
        .handle_quest_complete(player_guid, quest_giver_guid, quest_id, world)
        .await?;

    Ok(())
}

/// Handle CMSG_QUESTGIVER_CANCEL (0x190)
///
/// Sent when player clicks "Cancel" on a quest dialog.
/// Packet format: GUID (packed)
pub async fn handle_questgiver_cancel(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    info!("CMSG_QUESTGIVER_CANCEL: player={:?}", player_guid);

    // Send gossip complete to close the quest window
    use oxcore_shared::messages::gossip::SmsgGossipComplete;
    world
        .managers
        .broadcast_mgr
        .send_msg_to_player(player_guid, SmsgGossipComplete);

    Ok(())
}

/// Handle CMSG_QUESTGIVER_QUEST_AUTOLAUNCH (0x187)
///
/// Vanilla client sends this as a no-op.
pub async fn handle_questgiver_quest_auto_launch(
    _session: &WorldSession,
    _packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    Ok(())
}

/// Handle CMSG_QUESTLOG_REMOVE_QUEST (0x194)
///
/// Sent when player abandons a quest from their quest log.
/// Packet format: quest_slot (u8)
pub async fn handle_questlog_remove_quest(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let quest_slot = packet
        .read_u8()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest slot"))?;

    info!(
        "CMSG_QUESTLOG_REMOVE_QUEST: player={:?}, slot={}",
        player_guid, quest_slot
    );

    // Get the quest ID from the player's active quests at the specified slot
    let quest_id = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .and_then(|p| p.active_quests.get(quest_slot as usize).map(|q| q.quest_id));

    if let Some(quest_id) = quest_id {
        // Delegate to quest system to handle abandonment (DB + update fields)
        world
            .systems
            .quest
            .abandon_quest(player_guid, quest_id)
            .await?;
    } else {
        warn!(
            "Player {:?} tried to abandon quest at invalid slot {}",
            player_guid, quest_slot
        );
    }

    Ok(())
}

/// Handle CMSG_QUESTLOG_SWAP_QUEST (0x193)
///
/// Sent when player swaps quest positions in their quest log.
/// Packet format: slot1 (u8), slot2 (u8)
pub async fn handle_questlog_swap_quest(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let slot1 = packet
        .read_u8()
        .ok_or_else(|| anyhow::anyhow!("Failed to read slot1"))?;
    let slot2 = packet
        .read_u8()
        .ok_or_else(|| anyhow::anyhow!("Failed to read slot2"))?;

    info!(
        "CMSG_QUESTLOG_SWAP_QUEST: player={:?}, slot1={}, slot2={}",
        player_guid, slot1, slot2
    );

    // Validate slot range
    if slot1 as usize >= 20 || slot2 as usize >= 20 {
        warn!("Invalid quest slots: {} or {}", slot1, slot2);
        return Ok(());
    }

    // Swap quests in player's quest log
    world.managers.player_mgr.with_player_mut(player_guid, |p| {
        if slot1 as usize >= p.active_quests.len() || slot2 as usize >= p.active_quests.len() {
            return;
        }
        p.active_quests.swap(slot1 as usize, slot2 as usize);
    });

    info!(
        "Swapped quest slots {} and {} for player {:?}",
        slot1, slot2, player_guid
    );

    // Note: In vanilla WoW, the client handles quest log UI updates automatically
    // No packet needs to be sent for swap operations
    Ok(())
}

/// Handle CMSG_QUEST_CONFIRM_ACCEPT (0x19B)
///
/// Sent when player confirms accepting a quest (e.g., from a party share).
/// Packet format: quest_id (u32)
pub async fn handle_quest_confirm_accept(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let quest_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest ID"))?;

    info!(
        "CMSG_QUEST_CONFIRM_ACCEPT: player={:?}, quest={}",
        player_guid, quest_id
    );

    world
        .systems
        .quest
        .handle_quest_confirm_accept(player_guid, quest_id, world)
        .await?;

    Ok(())
}

/// Handle CMSG_QUESTGIVER_REQUEST_REWARD (0x18D)
///
/// Sent after the request-items dialog, before the reward selection dialog.
/// Packet format: GUID (packed), quest_id (u32)
pub async fn handle_questgiver_request_reward(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let quest_giver_guid = read_questgiver_guid(session.protocol(), packet)?;

    let quest_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest ID"))?;

    info!(
        "CMSG_QUESTGIVER_REQUEST_REWARD: player={:?}, npc={:?}, quest={}",
        player_guid, quest_giver_guid, quest_id
    );

    // Request-reward only opens the reward dialog. The final reward is handled
    // by CMSG_QUESTGIVER_CHOOSE_REWARD.
    world.systems.quest.handle_quest_reward_request(
        player_guid,
        quest_giver_guid,
        quest_id,
        world,
    )?;

    Ok(())
}

/// Handle CMSG_QUESTGIVER_CHOOSE_REWARD (0x18E)
///
/// Sent when player clicks "Complete" on the quest reward dialog.
/// This is the final step in quest turn-in where the player selects their reward.
/// Packet format: GUID (packed), quest_id (u32), reward_index (u32)
pub async fn handle_questgiver_choose_reward(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let quest_giver_guid = read_questgiver_guid(session.protocol(), packet)?;

    let quest_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest ID"))?;

    let reward_choice = packet.read_u32().unwrap_or(0);

    info!(
        "CMSG_QUESTGIVER_CHOOSE_REWARD: player={:?}, npc={:?}, quest={}, reward={}",
        player_guid, quest_giver_guid, quest_id, reward_choice
    );

    // Delegate to quest system to complete the quest and give rewards
    world
        .systems
        .quest
        .handle_quest_reward(
            player_guid,
            quest_giver_guid,
            quest_id,
            reward_choice,
            world,
        )
        .await?;

    Ok(())
}

/// Handle CMSG_PUSHQUESTTOPARTY (0x19D)
///
/// Sent when player clicks "Share Quest" in the quest log.
/// Iterates group members and sends quest details or rejection messages.
/// Packet format: quest_id (u32)
pub async fn handle_push_quest_to_party(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let quest_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest ID"))?;

    info!(
        "CMSG_PUSHQUESTTOPARTY: player={:?}, quest={}",
        player_guid, quest_id
    );

    let Some(quest) = world.systems.quest.manager.get_quest_template(quest_id) else {
        return Ok(());
    };

    let Some(group) = world.systems.group.get_player_group(player_guid) else {
        return Ok(());
    };

    for member in &group.members {
        if member.guid == player_guid {
            continue;
        }

        // Send "sharing quest" indicator to sharer
        let sharing = MsgQuestPushResult {
            sender_guid: member.guid,
            msg: oxcore_shared::game::quest::quest_share_msg::SHARING_QUEST,
        };
        world
            .managers
            .broadcast_mgr
            .send_msg_to_player(player_guid, sharing);

        // Distance + map check
        let sharer_pos = world.managers.player_mgr.get_position(player_guid);
        let member_pos = world.managers.player_mgr.get_position(member.guid);
        let sharer_map = world
            .managers
            .player_mgr
            .get_player(player_guid)
            .map(|p| p.map_id);
        let member_map = world
            .managers
            .player_mgr
            .get_player(member.guid)
            .map(|p| p.map_id);
        let same_map = sharer_map
            .zip(member_map)
            .map(|(a, b)| a == b)
            .unwrap_or(false);
        let in_range = sharer_pos
            .zip(member_pos)
            .map(|(sp, mp)| sp.is_within_range(&mp, QUEST_SHARE_DISTANCE))
            .unwrap_or(false);

        if !same_map || !in_range {
            let msg = MsgQuestPushResult {
                sender_guid: member.guid,
                msg: oxcore_shared::game::quest::quest_share_msg::TOO_FAR,
            };
            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(player_guid, msg);
            continue;
        }

        // Already completed?
        let status = world.systems.quest.get_quest_status(member.guid, quest_id);
        if status == QuestStatus::Complete {
            let msg = MsgQuestPushResult {
                sender_guid: member.guid,
                msg: oxcore_shared::game::quest::quest_share_msg::FINISH_QUEST,
            };
            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(player_guid, msg);
            continue;
        }

        // Already has quest (in progress)?
        if status == QuestStatus::Incomplete {
            let msg = MsgQuestPushResult {
                sender_guid: member.guid,
                msg: oxcore_shared::game::quest::quest_share_msg::HAVE_QUEST,
            };
            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(player_guid, msg);
            continue;
        }

        // Can take quest?
        if !world
            .systems
            .quest
            .can_take_quest(member.guid, &quest, world)
        {
            let msg = MsgQuestPushResult {
                sender_guid: member.guid,
                msg: oxcore_shared::game::quest::quest_share_msg::CANT_TAKE_QUEST,
            };
            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(player_guid, msg);
            continue;
        }

        // Log full?
        let log_full = world
            .managers
            .player_mgr
            .with_player(member.guid, |p| p.active_quests.len() >= MAX_QUEST_LOG_SIZE)
            .unwrap_or(true);
        if log_full {
            let msg = MsgQuestPushResult {
                sender_guid: member.guid,
                msg: oxcore_shared::game::quest::quest_share_msg::LOG_FULL,
            };
            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(player_guid, msg);
            continue;
        }

        // Already busy with another share?
        if world
            .managers
            .player_mgr
            .get_quest_share_info(member.guid)
            .is_some()
        {
            let msg = MsgQuestPushResult {
                sender_guid: member.guid,
                msg: oxcore_shared::game::quest::quest_share_msg::BUSY,
            };
            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(player_guid, msg);
            continue;
        }

        // All checks pass: send quest details to recipient
        let _ = world
            .systems
            .quest
            .send_quest_details(member.guid, player_guid, quest_id, world);

        // Set quest share info on recipient
        world.managers.player_mgr.set_quest_share_info(
            member.guid,
            QuestShareInfo {
                player_guid,
                quest_id,
            },
        );
    }

    Ok(())
}

/// Handle MSG_QUEST_PUSH_RESULT (0x276) — client→server direction
///
/// Sent by a player who received a shared quest to relay their response
/// (accept/decline/too_far/etc.) back to the original sharer.
/// Packet format: guid (packed — the responder), msg (u8)
pub async fn handle_quest_push_result(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let _sender_guid = packet
        .read_guid_for(session.protocol())
        .ok_or_else(|| anyhow::anyhow!("Failed to read sender GUID"))?;

    let msg = packet
        .read_u8()
        .ok_or_else(|| anyhow::anyhow!("Failed to read message"))?;

    info!(
        "MSG_QUEST_PUSH_RESULT: player={:?}, msg={}",
        player_guid, msg
    );

    let Some(share) = world.managers.player_mgr.get_quest_share_info(player_guid) else {
        return Ok(());
    };

    // Forward the response to the original sharer
    if world
        .managers
        .player_mgr
        .get_player(share.player_guid)
        .is_some()
    {
        let response = MsgQuestPushResult {
            sender_guid: player_guid,
            msg,
        };
        world
            .managers
            .broadcast_mgr
            .send_msg_to_player(share.player_guid, response);
    }

    world
        .managers
        .player_mgr
        .clear_quest_share_info(player_guid);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::core::session::WorldSession;
    use crate::game::broadcast_mgr::BroadcastManagerTrait;
    use crate::game::creature::{Creature, CreatureTemplate};
    use crate::game::npc::quest::system::QuestSystem;
    use crate::game::npc::quest::types::{QuestProgress, QuestTemplate};
    use crate::game::player::broadcaster::PlayerBroadcaster;
    use crate::game::player::Player;
    use oxcore_shared::database::characters::repositories::quest_repository::{
        MockQuestRepositoryTrait, QuestRepositoryTrait,
    };
    use oxcore_shared::database::Databases;
    use oxcore_shared::protocol::bitbuf::BitWriter;
    use oxcore_shared::protocol::{HighGuid, ObjectGuid, Position};
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    fn lazy_pool() -> sqlx::MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> crate::World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: lazy_pool(),
        });

        crate::World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn install_mock_quest_system(world: &mut crate::World) {
        let mut mock_repo = MockQuestRepositoryTrait::new();
        mock_repo
            .expect_delete_quest_status()
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let quest_repo: Arc<dyn QuestRepositoryTrait> = Arc::new(mock_repo);
        let broadcast_mgr: Arc<dyn BroadcastManagerTrait> = world.managers.broadcast_mgr.clone();
        let quest_system = Arc::new(QuestSystem::new(
            Arc::clone(&world.systems.quest.manager),
            quest_repo,
            broadcast_mgr,
            Arc::clone(&world.managers.player_mgr),
            Arc::clone(&world.managers.creature_mgr),
            Arc::clone(&world.managers.item_mgr),
            Arc::clone(&world.systems.inventory),
            Arc::clone(&world.systems.experience),
        ));

        Arc::get_mut(&mut world.systems)
            .expect("systems should be uniquely owned")
            .quest = quest_system;
    }

    fn test_player_guid() -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Player, 1)
    }

    fn test_creature_guid(entry: u32, counter: u32) -> ObjectGuid {
        ObjectGuid::new_creature(entry, counter)
    }

    #[test]
    fn modern_questgiver_status_reads_packed_guid128() {
        let expected = test_creature_guid(197, 42);
        let (high, low) = expected.to_guid128(1);
        let mut writer = BitWriter::new();
        writer.write_packed_guid_128(high, low);
        let mut packet = WorldPacket::new(Opcode::CMSG_QUESTGIVER_STATUS_QUERY);
        packet.write_bytes(&writer.into_bytes());

        assert_eq!(
            read_questgiver_guid(oxcore_shared::protocol::Protocol::Modern, &mut packet)
                .unwrap(),
            expected
        );
    }

    fn add_player(
        world: &mut crate::World,
    ) -> (
        Arc<WorldSession>,
        mpsc::UnboundedReceiver<oxcore_shared::protocol::WorldPacket>,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();
        let session = Arc::new(WorldSession::new(1, 1, "Tester".to_string(), 0, tx));
        let player_guid = test_player_guid();
        session.set_player_guid(Some(player_guid));
        world.session_mgr.add_session(Arc::clone(&session));
        world.session_mgr.register_player(session.id(), player_guid);

        let mut player = Player::new(player_guid, "Tester".to_string(), 0, 0, 0, 10, 1, 1, 0);
        player.set_broadcaster(Arc::new(PlayerBroadcaster::new(
            session.packet_tx(),
            player_guid,
        )));
        world.managers.player_mgr.add_player(player, 1);
        (session, rx)
    }

    fn add_creature(world: &mut crate::World, entry: u32, npc_flags: u32) -> ObjectGuid {
        add_creature_with_flags(world, entry, npc_flags, 0, 7)
    }

    fn add_creature_with_flags(
        world: &mut crate::World,
        entry: u32,
        npc_flags: u32,
        flags_extra: u32,
        creature_type: u8,
    ) -> ObjectGuid {
        let guid = test_creature_guid(entry, 1);
        let template = CreatureTemplate {
            entry,
            name: format!("Creature {entry}"),
            subname: None,
            min_level: 1,
            max_level: 1,
            faction: 35,
            model_id_1: 1,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra,
            creature_type,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            rank: 0,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
        };

        world.managers.creature_mgr.add_template(template.clone());
        world.managers.creature_mgr.add_creature(Creature::new(
            guid,
            entry,
            1,
            Position::default(),
            0,
            0,
            &template,
            1,
            None,
        ));
        guid
    }

    fn add_quest(world: &mut crate::World, quest_id: u32, creature_entry: Option<u32>) {
        let quest = QuestTemplate {
            id: quest_id,
            title: format!("Quest {quest_id}"),
            quest_level: 10,
            ..QuestTemplate::default()
        };
        world.systems.quest.manager.add_quest_template(quest);

        if let Some(entry) = creature_entry {
            world
                .systems
                .quest
                .manager
                .add_creature_quest_starter(entry, quest_id);
            world
                .systems
                .quest
                .manager
                .add_creature_quest_ender(entry, quest_id);
        }
    }

    fn read_packet(
        rx: &mut mpsc::UnboundedReceiver<oxcore_shared::protocol::WorldPacket>,
    ) -> oxcore_shared::protocol::WorldPacket {
        rx.try_recv().expect("expected a packet")
    }

    #[tokio::test]
    async fn questgiver_query_invalid_target_closes_gossip() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let npc_guid = add_creature(&mut world, 100, 0x0000_0002);
        add_quest(&mut world, 1, Some(200));

        let mut packet =
            oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_QUERY_QUEST);
        packet.write_guid(npc_guid);
        packet.write_u32(1);

        handle_questgiver_query_quest(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::SMSG_GOSSIP_COMPLETE);
    }

    #[tokio::test]
    async fn questgiver_query_valid_target_sends_details() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let npc_guid = add_creature(&mut world, 100, 0x0000_0002);
        add_quest(&mut world, 1, Some(100));

        let mut packet =
            oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_QUERY_QUEST);
        packet.write_guid(npc_guid);
        packet.write_u32(1);

        handle_questgiver_query_quest(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::SMSG_QUESTGIVER_QUEST_DETAILS);
    }

    #[tokio::test]
    async fn quest_query_sends_response_packet() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        add_quest(&mut world, 1, Some(100));

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUEST_QUERY);
        packet.write_u32(1);

        handle_quest_query(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::SMSG_QUEST_QUERY_RESPONSE);
    }

    #[tokio::test]
    async fn questgiver_cancel_sends_gossip_complete() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_CANCEL);
        handle_questgiver_cancel(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::SMSG_GOSSIP_COMPLETE);
    }

    #[tokio::test]
    async fn questgiver_autolaunch_is_noop() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);

        let mut packet =
            oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_QUEST_AUTOLAUNCH);
        handle_questgiver_quest_auto_launch(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn questlog_swap_valid_and_invalid_slots() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let player_guid = test_player_guid();
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.active_quests.push(QuestProgress::new(1));
                player.active_quests.push(QuestProgress::new(2));
            });

        let mut invalid =
            oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTLOG_SWAP_QUEST);
        invalid.write_u8(1);
        invalid.write_u8(25);
        handle_questlog_swap_quest(&session, &mut invalid, &world)
            .await
            .expect("handler should succeed");

        assert!(rx.try_recv().is_err());

        let mut valid = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTLOG_SWAP_QUEST);
        valid.write_u8(0);
        valid.write_u8(1);
        handle_questlog_swap_quest(&session, &mut valid, &world)
            .await
            .expect("handler should succeed");

        let player = world
            .managers
            .player_mgr
            .get_player(player_guid)
            .expect("player");
        assert_eq!(player.active_quests[0].quest_id, 2);
        assert_eq!(player.active_quests[1].quest_id, 1);
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn questlog_remove_quest_removes_slot_and_sends_update() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let player_guid = test_player_guid();
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.active_quests.push(QuestProgress::new(1));
                player.active_quests.push(QuestProgress::new(2));
            });

        let mut packet =
            oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTLOG_REMOVE_QUEST);
        packet.write_u8(0);
        handle_questlog_remove_quest(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let player = world
            .managers
            .player_mgr
            .get_player(player_guid)
            .expect("player");
        assert_eq!(player.active_quests.len(), 1);
        assert_eq!(player.active_quests[0].quest_id, 2);
        assert!(rx.try_recv().is_ok());
    }

    // --- Status query tests for hostile creature suppression ---

    fn add_creature_with_faction(
        world: &mut crate::World,
        entry: u32,
        npc_flags: u32,
        faction: u32,
    ) -> ObjectGuid {
        let guid = test_creature_guid(entry, 1);
        let template = CreatureTemplate {
            entry,
            name: format!("Creature {entry}"),
            subname: None,
            min_level: 1,
            max_level: 1,
            faction,
            model_id_1: 1,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra: 0,
            creature_type: 7,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            rank: 0,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
        };

        world.managers.creature_mgr.add_template(template.clone());
        world.managers.creature_mgr.add_creature(Creature::new(
            guid,
            entry,
            1,
            Position::default(),
            0,
            0,
            &template,
            1,
            None,
        ));
        guid
    }

    #[tokio::test]
    async fn questgiver_status_query_hostile_suppresses() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        // Faction 14 = Monster (hostile to all players)
        let npc_guid = add_creature_with_faction(&mut world, 100, 0x0000_0002, 14);
        add_quest(&mut world, 1, Some(100));

        let mut packet =
            oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_STATUS_QUERY);
        packet.write_guid(npc_guid);

        handle_questgiver_status_query(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        // Hostile creatures send DialogStatus::None — we verify a packet IS sent
        // (the handler sends it explicitly rather than silently returning)
        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::SMSG_QUESTGIVER_STATUS);
    }

    #[tokio::test]
    async fn questgiver_status_query_nonhostile_sends_available_status() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        // Faction 35 = Friendly to players (not in HOSTILE_TO_PLAYERS)
        let npc_guid = add_creature_with_faction(&mut world, 100, 0x0000_0002, 35);
        add_quest(&mut world, 1, Some(100));

        let mut packet =
            oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_STATUS_QUERY);
        packet.write_guid(npc_guid);

        handle_questgiver_status_query(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::SMSG_QUESTGIVER_STATUS);
    }

    #[tokio::test]
    async fn questgiver_status_query_unknown_guid_returns_without_packet() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let unknown_guid = ObjectGuid::new_creature(999, 1);

        let mut packet =
            oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_STATUS_QUERY);
        packet.write_guid(unknown_guid);

        handle_questgiver_status_query(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        assert!(
            rx.try_recv().is_err(),
            "no packet should be sent for unknown guid"
        );
    }

    // --- Hello handler tests ---

    #[tokio::test]
    async fn questgiver_hello_dead_creature_rejected() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_QUEST_GIVER);
        // Kill the creature
        world
            .managers
            .creature_mgr
            .with_creature_mut(npc_guid, |c| {
                c.current_health = 0;
            });

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        assert!(
            rx.try_recv().is_err(),
            "no gossip should open for dead creature"
        );
    }

    #[tokio::test]
    async fn questgiver_hello_no_npc_flags_rejected() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        // npc_flags = 0 means no interaction flags
        let npc_guid = add_creature(&mut world, 100, 0);

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        assert!(
            rx.try_recv().is_err(),
            "no gossip should open for creature without npc flags"
        );
    }

    #[tokio::test]
    async fn questgiver_hello_unknown_guid_rejected() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let unknown_guid = ObjectGuid::new_creature(999, 1);

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(unknown_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        assert!(
            rx.try_recv().is_err(),
            "no gossip should open for unknown guid"
        );
    }

    #[tokio::test]
    async fn questgiver_hello_clears_feign_death() {
        use crate::game::player::auras::aura::AuraFlags;
        use crate::game::player::auras::effects::AURA_FEIGN_DEATH;
        use crate::game::player::auras::Aura;

        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let player_guid = test_player_guid();
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_QUEST_GIVER);

        // Give the player a feign death aura (follows auction test pattern)
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.auras.container.add_aura(Aura::new(
                    AURA_FEIGN_DEATH,
                    player_guid,
                    0,
                    AURA_FEIGN_DEATH,
                    0,
                    1,
                    Some(10_000),
                    0,
                    1,
                    0,
                    AuraFlags::default(),
                ));
            });

        // Verify feign death is present before hello
        let has_feign = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
                player.auras.container.has_aura(AURA_FEIGN_DEATH)
            })
            .unwrap_or(false);
        assert!(has_feign, "feign death should be present before hello");

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        // Verify feign death is removed after hello
        let has_feign = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
                player.auras.container.has_aura(AURA_FEIGN_DEATH)
            })
            .unwrap_or(true);
        assert!(!has_feign, "feign death should be cleared after hello");

        // Should still receive a gossip packet since creature is valid
        let out = rx.try_recv();
        assert!(
            out.is_ok(),
            "valid questgiver should send gossip after hello"
        );
    }

    #[tokio::test]
    async fn questgiver_hello_normal_questgiver_sends_quest_details() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_QUEST_GIVER);
        add_quest(&mut world, 1, Some(100));

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::SMSG_QUESTGIVER_QUEST_DETAILS);
    }

    #[tokio::test]
    async fn quest_menu_keeps_independent_quest_when_its_next_quest_is_active() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (_session, _rx) = add_player(&mut world);
        let player_guid = test_player_guid();
        let creature_entry = 823; // Deputy Willem
        add_creature(&mut world, creature_entry, NPC_FLAG_QUEST_GIVER);

        let brotherhood = QuestTemplate {
            id: 18,
            title: "Brotherhood of Thieves".to_string(),
            quest_level: 4,
            next_quest_id: 3903,
            ..QuestTemplate::default()
        };
        let milly = QuestTemplate {
            id: 3903,
            title: "Milly Osworth".to_string(),
            quest_level: 4,
            ..QuestTemplate::default()
        };
        world.systems.quest.manager.add_quest_template(brotherhood);
        world.systems.quest.manager.add_quest_template(milly);
        world
            .systems
            .quest
            .manager
            .add_creature_quest_starter(creature_entry, 18);
        world
            .systems
            .quest
            .manager
            .add_creature_quest_starter(creature_entry, 3903);
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.active_quests.push(QuestProgress::new(3903));
            });

        let quest_ids: Vec<u32> = world
            .systems
            .quest
            .prepare_quest_menu(player_guid, creature_entry, &world)
            .into_iter()
            .map(|quest| quest.quest_id)
            .collect();

        assert!(quest_ids.contains(&18));
    }

    #[tokio::test]
    async fn questgiver_hello_gossip_flag_without_menu_sends_quest_details() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_GOSSIP | NPC_FLAG_QUEST_GIVER);
        add_quest(&mut world, 1, Some(100));

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::SMSG_QUESTGIVER_QUEST_DETAILS);
    }

    #[tokio::test]
    async fn gossip_hello_gossip_flag_without_menu_sends_quest_details() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_GOSSIP | NPC_FLAG_QUEST_GIVER);
        add_quest(&mut world, 1, Some(100));

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_GOSSIP_HELLO);
        packet.write_guid(npc_guid);

        crate::handlers::gossip_handler::handle_gossip_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::SMSG_QUESTGIVER_QUEST_DETAILS);
    }

    #[tokio::test]
    async fn questgiver_hello_pauses_non_civilian_non_totem_movement() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut _rx) = add_player(&mut world);
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_QUEST_GIVER);

        // Default test creature has flags_extra=0 (not civilian) and creature_type=7 (not totem)
        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        // Movement should be paused after hello
        let paused = world
            .managers
            .creature_mgr
            .with_creature_mut(npc_guid, |c| c.movement_paused)
            .expect("creature should exist");
        assert!(
            paused,
            "non-civilian, non-totem creature should have movement paused after hello"
        );
    }

    #[tokio::test]
    async fn questgiver_hello_does_not_pause_civilian_movement() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut _rx) = add_player(&mut world);
        let npc_guid = add_creature_with_flags(&mut world, 100, NPC_FLAG_QUEST_GIVER, 0x02, 7);

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        // Movement should NOT be paused for civilian creatures
        let paused = world
            .managers
            .creature_mgr
            .with_creature_mut(npc_guid, |c| c.movement_paused)
            .expect("creature should exist");
        assert!(!paused, "civilian creature movement should not be paused");
    }

    // --- Aura/Spell interrupt tests ---

    const CHANNELED_SPELL_ID: u32 = 9999;
    const INTERACT_INTERRUPT_FLAGS: u32 = 0x00000C00; // TALK | USE

    fn make_channeled_spell(channel_interrupt_flags: u32) -> crate::dbc::structures::SpellEntry {
        crate::dbc::structures::SpellEntry {
            id: CHANNELED_SPELL_ID,
            name: "Test Channel".to_string(),
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
            channel_interrupt_flags,
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
            effect: [0; 3],
            effect_die_sides: [0; 3],
            effect_base_dice: [0; 3],
            effect_dice_per_level: [0.0; 3],
            effect_real_points_per_level: [0.0; 3],
            effect_base_points: [0; 3],
            effect_bonus_coefficient: [0.0; 3],
            effect_mechanic: [0; 3],
            effect_implicit_target_a: [0; 3],
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

    fn add_channeled_cast(world: &mut crate::World, player_guid: ObjectGuid, spell_id: u32) {
        let cast = crate::game::player::spells::state::ActiveCast::new_channel(
            spell_id, None, 10_000, 10, false, 0.0, 0.0, 0.0,
        );
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.spells.set_current_spell(
                    crate::game::player::spells::state::CurrentSpellType::Channeled,
                    cast,
                );
            });
    }

    fn has_channeled_cast(world: &crate::World, player_guid: ObjectGuid) -> bool {
        world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
                player
                    .spells
                    .get_current_spell(
                        crate::game::player::spells::state::CurrentSpellType::Channeled,
                    )
                    .is_some()
            })
            .unwrap_or(false)
    }

    #[tokio::test]
    async fn questgiver_hello_cancels_channeled_spell_with_matching_flags() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut _rx) = add_player(&mut world);
        let player_guid = test_player_guid();
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_QUEST_GIVER);

        // Register a channeled spell with matching interrupt flags
        world
            .managers
            .spell_mgr
            .add_spell(make_channeled_spell(INTERACT_INTERRUPT_FLAGS));

        // Give the player a channeled cast
        add_channeled_cast(&mut world, player_guid, CHANNELED_SPELL_ID);
        assert!(
            has_channeled_cast(&world, player_guid),
            "channel should be active before hello"
        );

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        assert!(
            !has_channeled_cast(&world, player_guid),
            "channeled spell with matching interrupt flags should be cancelled after hello"
        );
    }

    #[tokio::test]
    async fn questgiver_hello_does_not_cancel_channeled_spell_without_matching_flags() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut _rx) = add_player(&mut world);
        let player_guid = test_player_guid();
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_QUEST_GIVER);

        // Register a channeled spell with NON-matching interrupt flags (DAMAGE flag = 0x2)
        world
            .managers
            .spell_mgr
            .add_spell(make_channeled_spell(0x2));

        // Give the player a channeled cast
        add_channeled_cast(&mut world, player_guid, CHANNELED_SPELL_ID);
        assert!(
            has_channeled_cast(&world, player_guid),
            "channel should be active before hello"
        );

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        assert!(
            has_channeled_cast(&world, player_guid),
            "channeled spell without matching interrupt flags should still be active after hello"
        );
    }

    #[tokio::test]
    async fn questgiver_hello_no_channeled_spell_does_not_error() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut _rx) = add_player(&mut world);
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_QUEST_GIVER);

        // No channeled spell set up on the player

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        // Handler should succeed without error even with no channel to cancel
        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed even with no active channel");
    }

    // ===== Push quest to party tests =====

    fn test_member_guid() -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Player, 2)
    }

    fn add_second_player(
        world: &mut crate::World,
    ) -> (
        Arc<WorldSession>,
        mpsc::UnboundedReceiver<oxcore_shared::protocol::WorldPacket>,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();
        let session = Arc::new(WorldSession::new(2, 2, "Member".to_string(), 0, tx));
        let player_guid = test_member_guid();
        session.set_player_guid(Some(player_guid));
        world.session_mgr.add_session(Arc::clone(&session));
        world.session_mgr.register_player(session.id(), player_guid);

        let mut player = Player::new(player_guid, "Member".to_string(), 0, 0, 0, 10, 1, 1, 0);
        player.set_broadcaster(Arc::new(PlayerBroadcaster::new(
            session.packet_tx(),
            player_guid,
        )));
        world.managers.player_mgr.add_player(player, 1);
        (session, rx)
    }

    fn add_group_with_members(
        world: &mut crate::World,
        leader_guid: ObjectGuid,
        member_guid: ObjectGuid,
    ) {
        let mut group =
            oxcore_shared::game::group::GroupData::new(1, leader_guid, "Tester".to_string());
        group
            .add_member(member_guid, "Member".to_string())
            .expect("add member");
        world.systems.group.add_group_test(group);
        world.systems.group.add_player_to_group_test(leader_guid, 1);
        world.systems.group.add_player_to_group_test(member_guid, 1);
    }

    #[tokio::test]
    async fn push_quest_to_party_not_logged_in() {
        let mut world = test_world();
        let session = Arc::new(WorldSession::new(
            99,
            99,
            "NoPlayer".to_string(),
            0,
            mpsc::unbounded_channel().0,
        ));
        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_PUSHQUESTTOPARTY);
        packet.write_u32(1);
        let result = handle_push_quest_to_party(&session, &mut packet, &world).await;
        assert!(result.is_err(), "should error when not logged in");
    }

    #[tokio::test]
    async fn push_quest_to_party_no_quest_template() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, _) = add_player(&mut world);
        let member_guid = test_member_guid();
        add_second_player(&mut world);
        add_group_with_members(&mut world, test_player_guid(), member_guid);

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_PUSHQUESTTOPARTY);
        packet.write_u32(999); // non-existent quest
        handle_push_quest_to_party(&session, &mut packet, &world)
            .await
            .expect("handler should succeed silently for unknown quest");
    }

    #[tokio::test]
    async fn push_quest_to_party_no_group() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, _) = add_player(&mut world);
        add_quest(&mut world, 1, None);

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_PUSHQUESTTOPARTY);
        packet.write_u32(1);
        handle_push_quest_to_party(&session, &mut packet, &world)
            .await
            .expect("handler should succeed silently when not in group");
    }

    #[tokio::test]
    async fn push_quest_to_party_successful_push() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let member_guid = test_member_guid();
        let (member_session, mut member_rx) = add_second_player(&mut world);
        add_group_with_members(&mut world, test_player_guid(), member_guid);
        add_quest(&mut world, 1, None);

        // Place both players on the same map and within distance
        let pos = oxcore_shared::protocol::Position::new(0.0, 0.0, 0.0, 0.0);
        world
            .managers
            .player_mgr
            .with_player_mut(test_player_guid(), |p| {
                p.movement.position = pos;
            });
        world.managers.player_mgr.with_player_mut(member_guid, |p| {
            p.movement.position = pos;
        });

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_PUSHQUESTTOPARTY);
        packet.write_u32(1);

        handle_push_quest_to_party(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        // Sharer should get SHARING_QUEST (0) message
        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);

        // Member should get quest details
        let member_out = member_rx.try_recv().expect("member should get a packet");
        assert_eq!(member_out.opcode(), Opcode::SMSG_QUESTGIVER_QUEST_DETAILS);

        // Member should have quest share info set
        let share = world.managers.player_mgr.get_quest_share_info(member_guid);
        assert!(share.is_some(), "member should have quest share info");
        assert_eq!(share.unwrap().quest_id, 1);
        assert_eq!(share.unwrap().player_guid, test_player_guid());
    }

    #[tokio::test]
    async fn push_quest_to_party_too_far() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let member_guid = test_member_guid();
        add_second_player(&mut world);
        add_group_with_members(&mut world, test_player_guid(), member_guid);
        add_quest(&mut world, 1, None);

        // Place member far away
        let pos = oxcore_shared::protocol::Position::new(0.0, 0.0, 0.0, 0.0);
        world
            .managers
            .player_mgr
            .with_player_mut(test_player_guid(), |p| {
                p.movement.position = pos;
            });
        world.managers.player_mgr.with_player_mut(member_guid, |p| {
            p.movement.position = oxcore_shared::protocol::Position::new(100.0, 0.0, 0.0, 0.0);
        });

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_PUSHQUESTTOPARTY);
        packet.write_u32(1);

        handle_push_quest_to_party(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        // Sharer should get SHARING_QUEST then TOO_FAR
        let out1 = read_packet(&mut rx);
        assert_eq!(out1.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);

        let out2 = read_packet(&mut rx);
        assert_eq!(out2.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);
    }

    #[tokio::test]
    async fn push_quest_to_party_already_completed() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let member_guid = test_member_guid();
        add_second_player(&mut world);
        add_group_with_members(&mut world, test_player_guid(), member_guid);
        add_quest(&mut world, 1, None);

        // Place both players on same map and within distance
        let pos = oxcore_shared::protocol::Position::new(0.0, 0.0, 0.0, 0.0);
        world
            .managers
            .player_mgr
            .with_player_mut(test_player_guid(), |p| {
                p.movement.position = pos;
            });
        world.managers.player_mgr.with_player_mut(member_guid, |p| {
            p.movement.position = pos;
        });

        // Mark quest as complete on member
        world
            .managers
            .player_mgr
            .with_player_mut(member_guid, |player| {
                let mut prog = crate::game::npc::quest::types::QuestProgress::new(1);
                prog.status = crate::game::npc::quest::types::QuestStatus::Complete;
                player.active_quests.push(prog);
            });

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_PUSHQUESTTOPARTY);
        packet.write_u32(1);

        handle_push_quest_to_party(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        // First packet: SHARING_QUEST, second packet: FINISH_QUEST
        let out1 = read_packet(&mut rx);
        assert_eq!(out1.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);
        let out2 = read_packet(&mut rx);
        assert_eq!(out2.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);
    }

    #[tokio::test]
    async fn push_quest_to_party_already_has_quest() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let member_guid = test_member_guid();
        add_second_player(&mut world);
        add_group_with_members(&mut world, test_player_guid(), member_guid);
        add_quest(&mut world, 1, None);

        let pos = oxcore_shared::protocol::Position::new(0.0, 0.0, 0.0, 0.0);
        world
            .managers
            .player_mgr
            .with_player_mut(test_player_guid(), |p| {
                p.movement.position = pos;
            });
        world.managers.player_mgr.with_player_mut(member_guid, |p| {
            p.movement.position = pos;
        });

        // Give member the quest as in-progress
        world
            .managers
            .player_mgr
            .with_player_mut(member_guid, |player| {
                player
                    .active_quests
                    .push(crate::game::npc::quest::types::QuestProgress::new(1));
            });

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_PUSHQUESTTOPARTY);
        packet.write_u32(1);

        handle_push_quest_to_party(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out1 = read_packet(&mut rx);
        assert_eq!(out1.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);
        let out2 = read_packet(&mut rx);
        assert_eq!(out2.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);
    }

    #[tokio::test]
    async fn push_quest_to_party_log_full() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let member_guid = test_member_guid();
        add_second_player(&mut world);
        add_group_with_members(&mut world, test_player_guid(), member_guid);
        add_quest(&mut world, 1, None);

        let pos = oxcore_shared::protocol::Position::new(0.0, 0.0, 0.0, 0.0);
        world
            .managers
            .player_mgr
            .with_player_mut(test_player_guid(), |p| {
                p.movement.position = pos;
            });
        world.managers.player_mgr.with_player_mut(member_guid, |p| {
            p.movement.position = pos;
        });

        // Fill member's quest log to max
        let max_slots = crate::game::npc::quest::types::MAX_QUEST_LOG_SIZE;
        world
            .managers
            .player_mgr
            .with_player_mut(member_guid, |player| {
                for i in 0..max_slots {
                    player
                        .active_quests
                        .push(crate::game::npc::quest::types::QuestProgress::new(
                            (100 + i) as u32,
                        ));
                }
            });

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_PUSHQUESTTOPARTY);
        packet.write_u32(1);

        handle_push_quest_to_party(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out1 = read_packet(&mut rx);
        assert_eq!(out1.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);
        let out2 = read_packet(&mut rx);
        assert_eq!(out2.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);
    }

    #[tokio::test]
    async fn push_quest_to_party_busy() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let member_guid = test_member_guid();
        add_second_player(&mut world);
        add_group_with_members(&mut world, test_player_guid(), member_guid);
        add_quest(&mut world, 1, None);

        let pos = oxcore_shared::protocol::Position::new(0.0, 0.0, 0.0, 0.0);
        world
            .managers
            .player_mgr
            .with_player_mut(test_player_guid(), |p| {
                p.movement.position = pos;
            });
        world.managers.player_mgr.with_player_mut(member_guid, |p| {
            p.movement.position = pos;
        });

        // Set existing quest share info on member (makes them busy)
        world.managers.player_mgr.set_quest_share_info(
            member_guid,
            QuestShareInfo {
                player_guid: test_player_guid(),
                quest_id: 99,
            },
        );

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::CMSG_PUSHQUESTTOPARTY);
        packet.write_u32(1);

        handle_push_quest_to_party(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out1 = read_packet(&mut rx);
        assert_eq!(out1.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);
        let out2 = read_packet(&mut rx);
        assert_eq!(out2.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);
    }

    // ===== Quest push result tests =====

    #[tokio::test]
    async fn quest_push_result_not_logged_in() {
        let world = test_world();
        let session = Arc::new(WorldSession::new(
            99,
            99,
            "NoPlayer".to_string(),
            0,
            mpsc::unbounded_channel().0,
        ));
        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::MSG_QUEST_PUSH_RESULT);
        packet.write_guid(test_player_guid());
        packet.write_u8(2); // accept
        let result = handle_quest_push_result(&session, &mut packet, &world).await;
        assert!(result.is_err(), "should error when not logged in");
    }

    #[tokio::test]
    async fn quest_push_result_no_share_info() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, _rx) = add_player(&mut world);

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::MSG_QUEST_PUSH_RESULT);
        packet.write_guid(test_player_guid());
        packet.write_u8(2); // accept

        handle_quest_push_result(&session, &mut packet, &world)
            .await
            .expect("handler should succeed silently when no share info");
    }

    #[tokio::test]
    async fn quest_push_result_forwards_to_sharer() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (sharer_session, mut sharer_rx) = add_player(&mut world);
        let member_guid = test_member_guid();
        let (member_session, _member_rx) = add_second_player(&mut world);

        // Set share info on member pointing to sharer
        world.managers.player_mgr.set_quest_share_info(
            member_guid,
            QuestShareInfo {
                player_guid: test_player_guid(),
                quest_id: 1,
            },
        );

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::MSG_QUEST_PUSH_RESULT);
        packet.write_guid(member_guid);
        packet.write_u8(oxcore_shared::game::quest::quest_share_msg::ACCEPT_QUEST);

        handle_quest_push_result(&member_session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        // Sharer should receive the forwarded response
        let out = read_packet(&mut sharer_rx);
        assert_eq!(out.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);

        // Share info should be cleared
        assert!(
            world
                .managers
                .player_mgr
                .get_quest_share_info(member_guid)
                .is_none(),
            "share info should be cleared"
        );
    }

    #[tokio::test]
    async fn quest_push_result_sharer_offline() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (_sharer_session, _) = add_player(&mut world);
        let member_guid = test_member_guid();
        let (member_session, _member_rx) = add_second_player(&mut world);

        // Set share info on member pointing to a GUID that has no session (offline)
        let offline_guid = ObjectGuid::new_without_entry(HighGuid::Player, 99);
        world.managers.player_mgr.set_quest_share_info(
            member_guid,
            QuestShareInfo {
                player_guid: offline_guid,
                quest_id: 1,
            },
        );

        let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::MSG_QUEST_PUSH_RESULT);
        packet.write_guid(member_guid);
        packet.write_u8(oxcore_shared::game::quest::quest_share_msg::DECLINE_QUEST);

        handle_quest_push_result(&member_session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        // Share info should still be cleared
        assert!(
            world
                .managers
                .player_mgr
                .get_quest_share_info(member_guid)
                .is_none(),
            "share info should be cleared even when sharer is offline"
        );
    }
}
