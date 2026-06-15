//! Quest packet handlers
//!
//! Slim handlers that parse packets and delegate to QuestSystem.

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::shared::messages::gossip::SmsgGossipComplete;
use crate::shared::messages::quest::{QuestObjectiveData, SmsgQuestQueryResponseV2};
use crate::shared::protocol::{Opcode, WorldPacket};
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::lua::{build_player_snapshot, execute_gossip_actions};
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
        zone_or_sort: quest.zone_or_sort,
        quest_type: quest.quest_type as u32,
        rep_objective_faction: quest.rep_objective_faction,
        rep_objective_value: quest.rep_objective_value,
        next_quest_in_chain: quest.next_quest_in_chain,
        rew_or_req_money: quest.rew_or_req_money,
        rew_money_max_level: quest.rew_money_max_level,
        rew_spell: quest.rew_spell,
        src_item_id: quest.src_item_id,
        quest_flags: crate::shared::messages::quest::QuestFlags(quest.quest_flags.bits()),
        rew_item_id: quest.rew_item_id,
        rew_item_count: quest.rew_item_count,
        rew_choice_item_id: quest.rew_choice_item_id,
        rew_choice_item_count: quest.rew_choice_item_count,
        point_map_id: quest.point_map_id,
        point_x: quest.point_x,
        point_y: quest.point_y,
        point_opt: quest.point_opt,
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
    let Some((entry, npc_flags)) = world
        .managers
        .creature_mgr
        .get_creature(quest_giver_guid)
        .map(|c| (c.entry, c.npc_flags))
    else {
        debug!(
            "Questgiver hello for unresolved or non-creature guid {:?}",
            quest_giver_guid
        );
        return Ok(());
    };

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

    let quest_data = if (npc_flags & 0x00000002) != 0 {
        Some(
            world
                .systems
                .quest
                .prepare_quest_menu(player_guid, entry, world)
                .into_iter()
                .map(|q| crate::shared::messages::GossipQuestData {
                    quest_id: q.quest_id,
                    icon: q.icon,
                    level: q.level,
                    title: q.title,
                })
                .collect(),
        )
    } else {
        None
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

    let related = world
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
        world.systems.quest.send_quest_details(
            player_guid,
            quest_giver_guid,
            quest.id,
            world,
        )?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::database::characters::repositories::quest_repository::{
        MockQuestRepositoryTrait, QuestRepositoryTrait,
    };
    use crate::shared::database::Databases;
    use crate::shared::protocol::{HighGuid, ObjectGuid, Position};
    use crate::world::config::Config;
    use crate::world::game::broadcast_mgr::BroadcastManagerTrait;
    use crate::world::core::session::WorldSession;
    use crate::world::game::creature::{Creature, CreatureTemplate};
    use crate::world::game::npc::quest::types::{QuestProgress, QuestTemplate};
    use crate::world::game::player::broadcaster::PlayerBroadcaster;
    use crate::world::game::player::Player;
    use crate::world::game::npc::quest::system::QuestSystem;
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    fn lazy_pool() -> sqlx::MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> crate::world::World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: lazy_pool(),
        });

        crate::world::World::new(databases, Arc::new(Config::default()), 50, PathBuf::from("."))
    }

    fn install_mock_quest_system(world: &mut crate::world::World) {
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

    fn add_player(world: &mut crate::world::World) -> (Arc<WorldSession>, mpsc::UnboundedReceiver<crate::shared::protocol::WorldPacket>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let session = Arc::new(WorldSession::new(1, 1, "Tester".to_string(), 0, tx));
        let player_guid = test_player_guid();
        session.set_player_guid(Some(player_guid));
        world.session_mgr.add_session(Arc::clone(&session));
        world.session_mgr.register_player(session.id(), player_guid);

        let mut player = Player::new(player_guid, "Tester".to_string(), 0, 0, 0, 10, 1, 1, 0);
        player.set_broadcaster(Arc::new(PlayerBroadcaster::new(session.packet_tx(), player_guid)));
        world.managers.player_mgr.add_player(player, 1);
        (session, rx)
    }

    fn add_creature(world: &mut crate::world::World, entry: u32, npc_flags: u32) -> ObjectGuid {
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

    fn add_quest(world: &mut crate::world::World, quest_id: u32, creature_entry: Option<u32>) {
        let quest = QuestTemplate {
            id: quest_id,
            title: format!("Quest {quest_id}"),
            quest_level: 10,
            ..QuestTemplate::default()
        };
        world.systems.quest.manager.add_quest_template(quest);

        if let Some(entry) = creature_entry {
            world.systems.quest.manager.add_creature_quest_starter(entry, quest_id);
            world.systems.quest.manager.add_creature_quest_ender(entry, quest_id);
        }
    }

    fn read_packet(rx: &mut mpsc::UnboundedReceiver<crate::shared::protocol::WorldPacket>) -> crate::shared::protocol::WorldPacket {
        rx.try_recv().expect("expected a packet")
    }

    #[tokio::test]
    async fn questgiver_query_invalid_target_closes_gossip() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let npc_guid = add_creature(&mut world, 100, 0x0000_0002);
        add_quest(&mut world, 1, Some(200));

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_QUERY_QUEST);
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

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_QUERY_QUEST);
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

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUEST_QUERY);
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

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_CANCEL);
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

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_QUEST_AUTOLAUNCH);
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
        world.managers.player_mgr.with_player_mut(player_guid, |player| {
            player.active_quests.push(QuestProgress::new(1));
            player.active_quests.push(QuestProgress::new(2));
        });

        let mut invalid = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTLOG_SWAP_QUEST);
        invalid.write_u8(1);
        invalid.write_u8(25);
        handle_questlog_swap_quest(&session, &mut invalid, &world)
            .await
            .expect("handler should succeed");

        assert!(rx.try_recv().is_err());

        let mut valid = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTLOG_SWAP_QUEST);
        valid.write_u8(0);
        valid.write_u8(1);
        handle_questlog_swap_quest(&session, &mut valid, &world)
            .await
            .expect("handler should succeed");

        let player = world.managers.player_mgr.get_player(player_guid).expect("player");
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
        world.managers.player_mgr.with_player_mut(player_guid, |player| {
            player.active_quests.push(QuestProgress::new(1));
            player.active_quests.push(QuestProgress::new(2));
        });

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTLOG_REMOVE_QUEST);
        packet.write_u8(0);
        handle_questlog_remove_quest(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let player = world.managers.player_mgr.get_player(player_guid).expect("player");
        assert_eq!(player.active_quests.len(), 1);
        assert_eq!(player.active_quests[0].quest_id, 2);
        assert!(rx.try_recv().is_ok());
    }
}
