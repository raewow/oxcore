//! Gossip System - business logic for NPC gossip
//!
//! Handles sending gossip menus to players, processing option selections,
//! filtering by conditions and NPC flags, and integration with quests.

use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::manager::GossipManager;
use super::types::{gossip_option, GossipMenuItem, DEFAULT_GOSSIP_MESSAGE};
use crate::shared::messages::gossip::{NpcTextOption, SmsgNpcTextUpdate};
use crate::shared::messages::{
    GossipOptionData, SmsgGossipComplete, SmsgGossipMessage, SmsgShowBank,
};
use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::{BroadcastManager, BroadcastManagerExt};
use crate::world::game::creature::CreatureManager;
use crate::world::game::player::PlayerManager;

/// Gossip system - handles gossip menu business logic
pub struct GossipSystem {
    manager: Arc<GossipManager>,
    broadcast_mgr: Arc<BroadcastManager>,
    player_mgr: Arc<PlayerManager>,
    creature_mgr: Arc<CreatureManager>,
}

impl GossipSystem {
    /// Create a new gossip system
    pub fn new(
        manager: Arc<GossipManager>,
        broadcast_mgr: Arc<BroadcastManager>,
        player_mgr: Arc<PlayerManager>,
        creature_mgr: Arc<CreatureManager>,
    ) -> Self {
        Self {
            manager,
            broadcast_mgr,
            player_mgr,
            creature_mgr,
        }
    }

    /// Send gossip menu to a player
    ///
    /// # Arguments
    /// * `player_guid` - The player's GUID
    /// * `npc_guid` - The NPC's GUID
    /// * `menu_id_override` - Optional menu ID to use instead of creature's default
    /// * `quest_data` - Optional quest data for quest giver NPCs
    pub async fn send_gossip_menu(
        &self,
        player_guid: ObjectGuid,
        npc_guid: ObjectGuid,
        menu_id_override: Option<u32>,
        quest_data: Option<Vec<crate::shared::messages::GossipQuestData>>,
    ) -> Result<()> {
        debug!(
            "Sending gossip menu to player {:?} from NPC {:?}",
            player_guid, npc_guid
        );

        // Get creature entry and npc_flags from CreatureManager
        let (entry, npc_flags) = {
            let creature = self
                .creature_mgr
                .get_creature(npc_guid)
                .ok_or_else(|| anyhow::anyhow!("Creature {:?} not found", npc_guid))?;
            (creature.entry, creature.npc_flags)
        };

        // Determine which menu to show
        let menu_id = menu_id_override
            .or_else(|| self.manager.get_creature_menu_id(entry))
            .unwrap_or(0);

        // Get the gossip menu - use default if not found
        let menus = self.manager.get_menus(menu_id);
        let default_menu = Arc::new(super::types::GossipMenu {
            entry: menu_id,
            text_id: 0, // Use 0 for quest-only menus (no gossip text)
            script_id: 0,
            condition_id: 0,
        });
        let menu = menus.first().cloned().unwrap_or(default_menu);

        // Get menu items
        let items = self.manager.get_menu_items(menu_id);

        // Filter items by conditions and NPC flags
        let filtered_items = self.filter_menu_items(&items, npc_flags, player_guid);

        // Build gossip options
        let options: Vec<GossipOptionData> = filtered_items
            .iter()
            .map(|item| GossipOptionData {
                index: item.id,
                icon: item.option_icon,
                coded: item.box_coded,
                money: item.box_money,
                text: item.option_text.clone(),
            })
            .collect();

        // Get quest items if NPC is a quest giver
        let quests = quest_data.unwrap_or_default();

        // Build and send the gossip message
        let msg = SmsgGossipMessage {
            source_guid: npc_guid,
            menu_id,
            text_id: menu.text_id,
            options,
            quests,
        };

        info!(
            "Sending SMSG_GOSSIP_MESSAGE: npc={:?}, menu_id={}, text_id={}, options={}, quests={}",
            npc_guid,
            menu_id,
            menu.text_id,
            msg.options.len(),
            msg.quests.len()
        );
        for opt in &msg.options {
            info!(
                "  gossip opt: index={} icon={} coded={} text={:?}",
                opt.index, opt.icon, opt.coded, opt.text
            );
        }

        // Debug: dump raw packet bytes
        let raw_pkt = crate::shared::messages::ToWorldPacket::to_world_packet(&msg);
        let raw_bytes = raw_pkt.data();
        let preview = raw_bytes.len().min(80);
        info!(
            "SMSG_GOSSIP_MESSAGE raw ({} bytes): {:02X?}",
            raw_bytes.len(),
            &raw_bytes[..preview]
        );

        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        // Proactively send SMSG_NPC_TEXT_UPDATE so the client can display the greeting
        // text without waiting for a separate CMSG_NPC_TEXT_QUERY round-trip.
        let text_id = menu.text_id;
        let npc_text_msg = if let Some(npc_text) = self.manager.get_npc_text(text_id) {
            let options = npc_text.options.map(|opt| {
                let bct = self.manager.get_broadcast_text(opt.broadcast_text_id);
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
            SmsgNpcTextUpdate {
                text_id,
                options: std::array::from_fn(|_| NpcTextOption::default()),
            }
        };
        self.broadcast_mgr
            .send_msg_to_player(player_guid, npc_text_msg);

        // Track the current gossip menu for this player
        if let Some(mut player) = self.player_mgr.get_player_mut(player_guid) {
            player.current_gossip_menu_id = Some(menu_id);
        }

        info!(
            "Sent gossip menu {} (text_id {}) to player {:?} from NPC {:?}",
            menu_id, menu.text_id, player_guid, npc_guid
        );

        Ok(())
    }

    /// Handle gossip option selection
    ///
    /// # Arguments
    /// * `player_guid` - The player's GUID
    /// * `npc_guid` - The NPC's GUID
    /// * `menu_id` - The current menu ID
    /// * `option_id` - The selected option ID
    pub async fn handle_option_select(
        &self,
        player_guid: ObjectGuid,
        npc_guid: ObjectGuid,
        menu_id: u32,
        option_id: u32,
    ) -> Result<()> {
        debug!(
            "Player {:?} selected option {} from menu {} on NPC {:?}",
            player_guid, option_id, menu_id, npc_guid
        );

        // Get menu items and find the selected one
        let items = self.manager.get_menu_items(menu_id);
        let option = items
            .iter()
            .find(|item| item.id == option_id)
            .ok_or_else(|| {
                anyhow::anyhow!("Gossip option {} not found in menu {}", option_id, menu_id)
            })?
            .clone();

        // Handle based on option type
        match option.option_id {
            gossip_option::GOSSIP => {
                if option.action_menu_id > 0 {
                    // Open sub-menu
                    self.send_gossip_menu(
                        player_guid,
                        npc_guid,
                        Some(option.action_menu_id as u32),
                        None,
                    )
                    .await?;
                } else if option.action_menu_id == 0 {
                    // Keep same menu (usually for scripted interactions)
                    self.send_gossip_menu(player_guid, npc_guid, Some(menu_id), None)
                        .await?;
                } else {
                    // Close gossip
                    self.send_gossip_complete(player_guid);
                }
            }
            gossip_option::VENDOR | gossip_option::ARMORER => {
                // Do NOT send SMSG_GOSSIP_COMPLETE — the vendor window replaces gossip.
                // vmangos: SendListInventory() with no CloseGossip() before it.
                info!(
                    "Vendor option selected for player {:?} from NPC {:?}",
                    player_guid, npc_guid
                );
            }
            gossip_option::TRAINER => {
                // Do NOT send SMSG_GOSSIP_COMPLETE — the trainer window replaces gossip.
                // vmangos: SendTrainerList() with no CloseGossip() before it.
                info!("Trainer option selected - would open trainer window");
            }
            gossip_option::TAXIVENDOR => {
                // Close gossip, open taxi
                self.send_gossip_complete(player_guid);
                // TODO: Open taxi window
                info!("Taxi option selected - would open taxi window");
            }
            gossip_option::BANKER => {
                info!(
                    "Banker option selected for player {:?} from NPC {:?}",
                    player_guid, npc_guid
                );
                self.broadcast_mgr.send_msg_to_player(
                    player_guid,
                    SmsgShowBank {
                        banker_guid: npc_guid,
                    },
                );
                if let Some(mut player) = self.player_mgr.get_player_mut(player_guid) {
                    player.current_gossip_menu_id = None;
                }
            }
            gossip_option::AUCTIONEER => {
                // Close gossip, open auction
                self.send_gossip_complete(player_guid);
                // TODO: Open auction window
                info!("Auctioneer option selected - would open auction window");
            }
            gossip_option::SPIRITHEALER => {
                // Close gossip, handle spirit healer
                self.send_gossip_complete(player_guid);
                // TODO: Handle spirit healer resurrection
                info!("Spirit healer option selected");
            }
            gossip_option::INNKEEPER => {
                // Close gossip, bind home
                self.send_gossip_complete(player_guid);
                // TODO: Bind home location
                info!("Innkeeper option selected - would bind home");
            }
            gossip_option::QUESTGIVER => {
                // Handle quest interaction
                self.send_gossip_complete(player_guid);
                // TODO: Open quest dialog
                info!("Quest giver option selected");
            }
            _ => {
                // Unknown option, just close
                warn!("Unknown gossip option type: {}", option.option_id);
                self.send_gossip_complete(player_guid);
            }
        }

        // Execute action script if defined
        if option.action_script_id > 0 {
            // TODO: Execute DB script
            info!("Would execute script {}", option.action_script_id);
        }

        // Show point of interest if defined
        if option.action_poi_id > 0 {
            // TODO: Send SMSG_GOSSIP_POI
            info!("Would show POI {}", option.action_poi_id);
        }

        Ok(())
    }

    /// Send gossip complete (close window)
    pub fn send_gossip_complete(&self, player_guid: ObjectGuid) {
        self.broadcast_mgr
            .send_msg_to_player(player_guid, SmsgGossipComplete);

        // Clear the current gossip menu for this player
        if let Some(mut player) = self.player_mgr.get_player_mut(player_guid) {
            player.current_gossip_menu_id = None;
        }
    }

    /// Filter menu items based on conditions and NPC flags
    fn filter_menu_items(
        &self,
        items: &[GossipMenuItem],
        npc_flags: u32,
        _player_guid: ObjectGuid,
    ) -> Vec<GossipMenuItem> {
        let mut filtered = Vec::new();

        for item in items {
            // Check NPC flag requirement
            if item.npc_option_npcflag > 0 {
                if (npc_flags & item.npc_option_npcflag) == 0 {
                    continue; // NPC doesn't have required flags
                }
            }

            // Check condition (if defined)
            if item.condition_id > 0 {
                // TODO: Check condition via ConditionSystem
                // For now, assume condition passes
                let condition_passes = true;
                if !condition_passes {
                    continue;
                }
            }

            // TODO: Check class/race requirements for trainer options
            // TODO: Check level requirements

            filtered.push(item.clone());
        }

        filtered
    }

    /// Get localized text for a menu option
    pub fn get_option_text(&self, item: &GossipMenuItem, _locale: u8) -> String {
        // First try broadcast text
        if item.option_broadcast_text > 0 {
            if let Some(text) = self
                .manager
                .get_localized_text(item.option_broadcast_text, false)
            {
                return text;
            }
        }

        // Fall back to raw text
        item.option_text.clone()
    }

    /// Get NPC text for a menu
    pub fn get_npc_text(&self, text_id: u32) -> Option<String> {
        if text_id == DEFAULT_GOSSIP_MESSAGE {
            return None;
        }

        let npc_text = self.manager.get_npc_text(text_id)?;

        // Select a text option based on probabilities
        let mut selected_id = 0;
        for option in npc_text.options.iter() {
            if option.probability > 0.0 {
                selected_id = option.broadcast_text_id;
                break; // Simplified - always picks first valid
            }
        }

        if selected_id > 0 {
            self.manager.get_localized_text(selected_id, false)
        } else {
            None
        }
    }

    /// Get option details for a menu item
    /// Returns the GossipMenuItem if found, None otherwise
    pub fn get_option_details(&self, menu_id: u32, option_id: u32) -> Option<GossipMenuItem> {
        let items = self.manager.get_menu_items(menu_id);
        items.into_iter().find(|item| item.id == option_id)
    }

    /// Check if a creature has a gossip menu defined
    /// Returns true if the creature has a non-default gossip menu
    pub fn has_gossip_menu(&self, entry: u32) -> bool {
        // Check if creature has a specific menu defined
        if let Some(menu_id) = self.manager.get_creature_menu_id(entry) {
            // Check if this menu has any items
            let items = self.manager.get_menu_items(menu_id);
            !items.is_empty()
        } else {
            false
        }
    }

    /// Initialize the gossip system
    pub async fn init(&self) -> Result<()> {
        Ok(())
    }

    /// Shutdown the gossip system
    pub async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
