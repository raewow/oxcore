//! Quest packet handlers
//!
//! Slim handlers that parse packets and delegate to QuestSystem.

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::shared::protocol::{Opcode, WorldPacket};
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::session::WorldSession;
use crate::world::World;

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

    let quest_giver_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest giver GUID"))?;

    debug!(
        "CMSG_QUESTGIVER_STATUS_QUERY: player={:?}, npc={:?}",
        player_guid, quest_giver_guid
    );

    // Delegate to quest system
    world
        .systems
        .quest
        .send_quest_giver_status(player_guid, quest_giver_guid, world);

    Ok(())
}

/// Handle CMSG_QUESTGIVER_HELLO (0x184)
///
/// Sent when player right-clicks a quest giver NPC.
/// This delegates to the gossip system to provide a unified interaction flow.
/// Packet format: GUID (packed)
pub async fn handle_questgiver_hello(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let quest_giver_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest giver GUID"))?;

    info!(
        "CMSG_QUESTGIVER_HELLO: player={:?}, npc={:?}",
        player_guid, quest_giver_guid
    );

    // Get creature entry and npc_flags
    let (entry, npc_flags) = world
        .managers
        .creature_mgr
        .get_creature(quest_giver_guid)
        .map(|c| (c.entry, c.npc_flags))
        .unwrap_or((0, 0));

    // Check if NPC also has GOSSIP flag - if so, let gossip hello handle it
    let has_gossip_flag = (npc_flags & 0x00000001) != 0;

    if has_gossip_flag {
        // NPC has both QUESTGIVER and GOSSIP flags - use gossip system
        // This prevents duplicate handling
        info!("NPC has GOSSIP flag, delegating to gossip system");
        return Ok(());
    }

    // NPC is a pure quest giver (no GOSSIP flag) - handle directly
    // Prepare quest data
    let quest_data = world
        .systems
        .quest
        .prepare_quest_menu(player_guid, entry, world);

    // Check if we should auto-display a single quest
    let quest_count = quest_data.len();

    if quest_count == 1 {
        // Auto-display the single quest directly
        if let Some(quest) = quest_data.first() {
            let quest_id = quest.quest_id;
            info!(
                "Auto-displaying single quest {} for player {:?} from NPC {:?}",
                quest_id, player_guid, quest_giver_guid
            );

            // Check quest status to determine which dialog to show
            use crate::world::game::npc::quest::types::QuestStatus;
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
                        .handle_quest_complete(player_guid, quest_giver_guid, quest_id, world)
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
                        .handle_quest_complete(player_guid, quest_giver_guid, quest_id, world)
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
                        quest_giver_guid,
                        quest_id,
                        world,
                    )?;
                }
            }
            return Ok(());
        }
    }

    // Send quest list using the correct SMSG_QUESTGIVER_QUEST_LIST packet
    world
        .systems
        .quest
        .send_quest_giver_quest_list(player_guid, quest_giver_guid, entry, world)?;

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

    let quest_giver_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest giver GUID"))?;

    let quest_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest ID"))?;

    info!(
        "CMSG_QUESTGIVER_QUERY_QUEST: player={:?}, npc={:?}, quest={}",
        player_guid, quest_giver_guid, quest_id
    );

    // Check quest status to determine which dialog to show
    use crate::world::game::npc::quest::types::QuestStatus;
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
                .handle_quest_complete(player_guid, quest_giver_guid, quest_id, world)
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
                .handle_quest_complete(player_guid, quest_giver_guid, quest_id, world)
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
                quest_giver_guid,
                quest_id,
                world,
            )?;
        }
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

    let quest_giver_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest giver GUID"))?;

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

    let quest_giver_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest giver GUID"))?;

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
    use crate::shared::messages::gossip::SmsgGossipComplete;
    world
        .managers
        .broadcast_mgr
        .send_msg_to_player(player_guid, SmsgGossipComplete);

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

    // Check if quest exists
    let Some(quest) = world.systems.quest.manager.get_quest_template(quest_id) else {
        warn!("Quest {} not found", quest_id);
        return Ok(());
    };

    // Check if player can take quest
    if !world
        .systems
        .quest
        .can_take_quest(player_guid, &quest, world)
    {
        warn!("Player {:?} cannot accept quest {}", player_guid, quest_id);
        return Ok(());
    }

    warn!(
        "CMSG_QUEST_CONFIRM_ACCEPT for quest {} from {:?} ignored: party quest sharing state is not implemented yet",
        quest_id, player_guid
    );
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

    let quest_giver_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest giver GUID"))?;

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

    let quest_giver_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read quest giver GUID"))?;

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
