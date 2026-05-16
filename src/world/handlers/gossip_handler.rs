//! Gossip packet handlers
//!
//! Slim handlers that parse packets and delegate to GossipSystem.

use anyhow::Result;
use tracing::{debug, info};

use crate::shared::protocol::{Opcode, WorldPacket};
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::lua::{build_player_snapshot, execute_gossip_actions};
use crate::world::core::session::WorldSession;
use crate::world::World;

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

    // CMSG_GOSSIP_HELLO uses unpacked GUID (8 bytes)
    let npc_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read NPC GUID"))?;

    info!(
        "CMSG_GOSSIP_HELLO: player={:?}, npc={:?}",
        player_guid, npc_guid
    );

    // Get creature info to check if it's a quest giver
    let (entry, npc_flags) = world
        .managers
        .creature_mgr
        .get_creature(npc_guid)
        .map(|c| (c.entry, c.npc_flags))
        .unwrap_or((0, 0));

    info!(
        "NPC entry={}, npc_flags=0x{:08X}, has_gossip={}, has_quest={}, has_vendor={}",
        entry,
        npc_flags,
        (npc_flags & 0x00000001) != 0,
        (npc_flags & 0x00000002) != 0,
        (npc_flags & 0x00000080) != 0
    );

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
                use crate::world::game::player::death::DeathState;
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

    // Check if we should auto-display a single quest (like world implementation)
    // Conditions: Has exactly 1 quest, is not a vendor, and either:
    //   - NPC has no GOSSIP flag (quest pickup), OR
    //   - Quest is complete (quest turn-in)
    let has_gossip_flag = (npc_flags & 0x00000001) != 0;
    let has_vendor_flag = (npc_flags & 0x00000080) != 0;
    let quest_count = quest_data.as_ref().map(|q| q.len()).unwrap_or(0);

    // Determine if we should auto-display the quest
    let should_auto_display = if quest_count == 1 && !has_vendor_flag {
        if let Some(ref quests) = quest_data {
            if let Some(quest) = quests.first() {
                let quest_id = quest.quest_id;
                use crate::world::game::npc::quest::types::QuestStatus;
                let quest_status = world.systems.quest.get_quest_status(player_guid, quest_id);

                // Auto-display if: no gossip flag (pickup) OR quest is complete (turn-in)
                !has_gossip_flag || quest_status == QuestStatus::Complete
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

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
        let has_gossip_menu = world.systems.gossip.has_gossip_menu(entry);

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
            crate::world::handlers::trainer_handler::send_trainer_list(
                player_guid,
                npc_guid,
                world,
            )
            .await?;
            return Ok(());
        }
    }

    // Convert quest data to gossip format
    let gossip_quests = quest_data.map(|quests| {
        quests
            .into_iter()
            .map(|q| crate::shared::messages::GossipQuestData {
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

    // CMSG_GOSSIP_SELECT_OPTION uses unpacked GUID (8 bytes)
    let npc_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read NPC GUID"))?;

    // Client sends option_id (called gossipListId in MaNGOS), NOT menu_id
    let option_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read option ID"))?;

    // Optional coded text (for input boxes)
    let _coded_text = packet.read_cstring();

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
        use crate::world::game::npc::gossip::types::gossip_option;
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
                crate::world::handlers::trainer_handler::send_trainer_list(
                    player_guid,
                    npc_guid,
                    world,
                )
                .await?;
            }
            _ => {
                // Other options are handled by the gossip system
            }
        }
    }

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

    let text_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read text ID"))?;

    // CMSG_NPC_TEXT_QUERY uses unpacked GUID (8 bytes)
    let _npc_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read NPC GUID"))?;

    debug!("CMSG_NPC_TEXT_QUERY: text_id={}", text_id);

    use crate::shared::messages::gossip::{NpcTextOption, SmsgNpcTextUpdate};

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
        // Text not found - send default empty response (matches vmangos fallback)
        let options = std::array::from_fn(|_| NpcTextOption::default());
        SmsgNpcTextUpdate { text_id, options }
    };

    world
        .managers
        .broadcast_mgr
        .send_msg_to_player(player_guid, msg);

    Ok(())
}
