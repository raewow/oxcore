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
use crate::world::game::creature::ai::{is_hostile_faction, is_npc, NPC_FLAG_QUEST_GIVER};
use crate::world::game::common::player_constants::get_faction_for_race;
use crate::world::game::player::auras::effects::AURA_FEIGN_DEATH;
use crate::world::game::player::spells::state::CurrentSpellType;
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
            let msg = crate::shared::messages::quest::SmsgQuestgiverStatus {
                guid: quest_giver_guid,
                status: crate::shared::messages::quest::DialogStatus::None,
            };
            world.managers.broadcast_mgr.send_msg_to_player(player_guid, msg);
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

    // Resolve creature questgiver with interaction validation.
    // Must exist, be alive, and have NPC interaction flags (gossip or quest giver).
    let (entry, npc_flags, creature_type) = {
        let Some(creature) = world
            .managers
            .creature_mgr
            .get_creature(quest_giver_guid)
        else {
            debug!(
                "Questgiver hello for unresolved or non-creature guid {:?}",
                quest_giver_guid
            );
            return Ok(());
        };

        if !creature.is_alive() {
            debug!(
                "Questgiver {:?} is dead, rejecting hello",
                quest_giver_guid
            );
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
            world.managers.creature_mgr.with_creature_mut(
                quest_giver_guid,
                |c| {
                    c.pause_out_of_combat_movement();
                },
            );
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
                        world
                            .managers
                            .spell_mgr
                            .get(cast.spell_id)
                            .filter(|entry| {
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

    let quest_data = if (npc_flags & NPC_FLAG_QUEST_GIVER) != 0 {
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
        add_creature_with_flags(world, entry, npc_flags, 0, 7)
    }

    fn add_creature_with_flags(
        world: &mut crate::world::World,
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

    // --- Status query tests for hostile creature suppression ---

    fn add_creature_with_faction(
        world: &mut crate::world::World,
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

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_STATUS_QUERY);
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

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_STATUS_QUERY);
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

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_STATUS_QUERY);
        packet.write_guid(unknown_guid);

        handle_questgiver_status_query(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        assert!(rx.try_recv().is_err(), "no packet should be sent for unknown guid");
    }

    // --- Hello handler tests ---

    #[tokio::test]
    async fn questgiver_hello_dead_creature_rejected() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_QUEST_GIVER);
        // Kill the creature
        world.managers.creature_mgr.with_creature_mut(npc_guid, |c| {
            c.current_health = 0;
        });

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        assert!(rx.try_recv().is_err(), "no gossip should open for dead creature");
    }

    #[tokio::test]
    async fn questgiver_hello_no_npc_flags_rejected() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        // npc_flags = 0 means no interaction flags
        let npc_guid = add_creature(&mut world, 100, 0);

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        assert!(rx.try_recv().is_err(), "no gossip should open for creature without npc flags");
    }

    #[tokio::test]
    async fn questgiver_hello_unknown_guid_rejected() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let unknown_guid = ObjectGuid::new_creature(999, 1);

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(unknown_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        assert!(rx.try_recv().is_err(), "no gossip should open for unknown guid");
    }

    #[tokio::test]
    async fn questgiver_hello_clears_feign_death() {
        use crate::world::game::player::auras::Aura;
        use crate::world::game::player::auras::effects::AURA_FEIGN_DEATH;
        use crate::world::game::player::auras::aura::AuraFlags;

        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let player_guid = test_player_guid();
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_QUEST_GIVER);

        // Give the player a feign death aura (follows auction test pattern)
        world.managers.player_mgr.with_player_mut(player_guid, |player| {
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
        let has_feign = world.managers.player_mgr.with_player(player_guid, |player| {
            player.auras.container.has_aura(AURA_FEIGN_DEATH)
        }).unwrap_or(false);
        assert!(has_feign, "feign death should be present before hello");

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        // Verify feign death is removed after hello
        let has_feign = world.managers.player_mgr.with_player(player_guid, |player| {
            player.auras.container.has_aura(AURA_FEIGN_DEATH)
        }).unwrap_or(true);
        assert!(!has_feign, "feign death should be cleared after hello");

        // Should still receive a gossip packet since creature is valid
        let out = rx.try_recv();
        assert!(out.is_ok(), "valid questgiver should send gossip after hello");
    }

    #[tokio::test]
    async fn questgiver_hello_normal_questgiver_sends_gossip() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_QUEST_GIVER);
        add_quest(&mut world, 1, Some(100));

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::SMSG_GOSSIP_MESSAGE);
    }

    #[tokio::test]
    async fn questgiver_hello_pauses_non_civilian_non_totem_movement() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut _rx) = add_player(&mut world);
        let npc_guid = add_creature(&mut world, 100, NPC_FLAG_QUEST_GIVER);

        // Default test creature has flags_extra=0 (not civilian) and creature_type=7 (not totem)
        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
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
        assert!(paused, "non-civilian, non-totem creature should have movement paused after hello");
    }

    #[tokio::test]
    async fn questgiver_hello_does_not_pause_civilian_movement() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut _rx) = add_player(&mut world);
        let npc_guid = add_creature_with_flags(&mut world, 100, NPC_FLAG_QUEST_GIVER, 0x02, 7);

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
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

    fn make_channeled_spell(channel_interrupt_flags: u32) -> crate::world::dbc::structures::SpellEntry {
        crate::world::dbc::structures::SpellEntry {
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

    fn add_channeled_cast(world: &mut crate::world::World, player_guid: ObjectGuid, spell_id: u32) {
        let cast = crate::world::game::player::spells::state::ActiveCast::new_channel(
            spell_id,
            None,
            10_000,
            10,
            false,
            0.0,
            0.0,
            0.0,
        );
        world.managers.player_mgr.with_player_mut(player_guid, |player| {
            player.spells.set_current_spell(
                crate::world::game::player::spells::state::CurrentSpellType::Channeled,
                cast,
            );
        });
    }

    fn has_channeled_cast(world: &crate::world::World, player_guid: ObjectGuid) -> bool {
        world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
                player
                    .spells
                    .get_current_spell(crate::world::game::player::spells::state::CurrentSpellType::Channeled)
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
        world.managers.spell_mgr.add_spell(make_channeled_spell(INTERACT_INTERRUPT_FLAGS));

        // Give the player a channeled cast
        add_channeled_cast(&mut world, player_guid, CHANNELED_SPELL_ID);
        assert!(has_channeled_cast(&world, player_guid), "channel should be active before hello");

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
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
        world.managers.spell_mgr.add_spell(make_channeled_spell(0x2));

        // Give the player a channeled cast
        add_channeled_cast(&mut world, player_guid, CHANNELED_SPELL_ID);
        assert!(has_channeled_cast(&world, player_guid), "channel should be active before hello");

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
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

        let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(npc_guid);

        // Handler should succeed without error even with no channel to cancel
        handle_questgiver_hello(&session, &mut packet, &world)
            .await
            .expect("handler should succeed even with no active channel");
    }
}
